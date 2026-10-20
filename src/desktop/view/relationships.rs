//! A ticket tab's relationships panel (design, Item Inspection,
//! Relationships): four groups, Dependencies, Dependents, Parent and
//! Children, then the ticket's relationship problems.
//!
//! Each row gives the related ticket's short code and title, as the
//! library returned them, its lifecycle and its resolution, each in text,
//! and its ID. A row with a resolved target is a tab stop, so Tab within
//! the content region reaches it; a click, Enter or Space on it opens the
//! ticket by its ID in the tab's repository. An unresolved target shows
//! the ID it names and says, in fixed text, that it cannot be opened; it
//! offers no action and no focus. A document's tab, and a nonconforming
//! entry's, have no panel.

use gpui_kit::{
    AnyElement, ClickEvent, Context, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    Role, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, h_flex, v_flex},
    div,
};
use manyhands::repository::ProblemDto;

use super::{
    item::{ItemPane, text},
    tickets::lifecycle_label,
};
use crate::desktop::model::{
    navigation::OpenTarget,
    relationships::{
        CANNOT_OPEN, CHILDREN, DEPENDENCIES, DEPENDENTS, NONE, PARENT, PROBLEMS, RELATIONSHIPS,
        RelatedRow, resolution_label,
    },
};

/// A row of the panel was activated: open its ticket, by ID, in a tab.
pub(crate) struct OpenRelated(pub OpenTarget);

impl EventEmitter<OpenRelated> for ItemPane {}

/// How one group's elements are named: the group by `group`, its rows by
/// `(row, ix)`, and each row's cells by `rel-{cell}-{ix}-{name}`.
struct Group {
    group: &'static str,
    name: &'static str,
    row: &'static str,
    cell: &'static str,
}

const DEPENDENCY_GROUP: Group = Group {
    group: "rel-dependencies",
    name: DEPENDENCIES,
    row: "rel-dependency",
    cell: "dependency",
};
const DEPENDENT_GROUP: Group = Group {
    group: "rel-dependents",
    name: DEPENDENTS,
    row: "rel-dependent",
    cell: "dependent",
};
const PARENT_GROUP: Group = Group {
    group: "rel-parent-group",
    name: PARENT,
    row: "rel-parent",
    cell: "parent",
};
const CHILD_GROUP: Group = Group {
    group: "rel-children",
    name: CHILDREN,
    row: "rel-child",
    cell: "child",
};

/// A row's text for assistive technology: what it shows, in order.
fn summary(row: &RelatedRow) -> String {
    [
        row.slug.clone(),
        row.title.clone(),
        row.lifecycle
            .map(|lifecycle| lifecycle_label(lifecycle).to_owned()),
        Some(resolution_label(row.resolved).to_owned()),
        Some(row.id.clone()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ")
}

impl ItemPane {
    /// The panel, for a ticket whose item read has landed; `None` for a
    /// document or a nonconforming entry.
    pub(super) fn relationships_panel(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let relationships = self.model.relationships()?;
        let dependency_rows = relationships.dependency_rows();
        let dependent_rows = relationships.dependent_rows();
        let parent_rows: Vec<RelatedRow> = relationships.parent_row().into_iter().collect();
        let child_rows = relationships.child_rows();
        let problems = relationships.problems().to_vec();
        // A group read by its own read says how that read stands, and
        // says None only once it has landed.
        let dependents_known = relationships.dependents().rows().is_some();
        let children_known = relationships.children().rows().is_some();
        let dependents_status = self
            .status("dependents", relationships.dependents(), cx)
            .into_any_element();
        let children_status = self
            .status("children", relationships.children(), cx)
            .into_any_element();
        let dependencies = self.group(&DEPENDENCY_GROUP, None, &dependency_rows, true, window, cx);
        let dependents = self.group(
            &DEPENDENT_GROUP,
            Some(dependents_status),
            &dependent_rows,
            dependents_known,
            window,
            cx,
        );
        let parent = self.group(&PARENT_GROUP, None, &parent_rows, true, window, cx);
        let children = self.group(
            &CHILD_GROUP,
            Some(children_status),
            &child_rows,
            children_known,
            window,
            cx,
        );
        Some(
            v_flex()
                .id("relationships")
                .test_support()
                .aria_label(RELATIONSHIPS)
                .gap_2()
                .child(div().text_sm().child(RELATIONSHIPS))
                .child(dependencies)
                .child(dependents)
                .child(parent)
                .child(children)
                .children(self.relationship_problems(&problems, cx))
                .into_any_element(),
        )
    }

    /// One group: its name, the state of its read when it has one, then
    /// its rows, or None once it is known to have none.
    fn group(
        &mut self,
        group: &Group,
        status: Option<AnyElement>,
        rows: &[RelatedRow],
        known: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let empty = (known && rows.is_empty()).then(|| {
            text(format!("rel-{}-none", group.cell), NONE)
                .text_xs()
                .text_color(muted)
        });
        let rows: Vec<AnyElement> = rows
            .iter()
            .enumerate()
            .map(|(ix, row)| self.related_row(group, ix, row, window, cx))
            .collect();
        v_flex()
            .id(group.group)
            .test_support()
            .aria_label(group.name)
            .gap_1()
            .child(div().text_xs().text_color(muted).child(group.name))
            .children(status)
            .children(rows)
            .children(empty)
    }

    /// One related ticket. A resolved one is a tab stop and opens its
    /// ticket when activated; the focused one is outlined.
    fn related_row(
        &mut self,
        group: &Group,
        ix: usize,
        row: &RelatedRow,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (muted, border, ring) = (theme.muted_foreground, theme.border, theme.ring);
        let cell = |name: &str| format!("rel-{}-{ix}-{name}", group.cell);
        let mut element = h_flex()
            .id((group.row, ix))
            .test_support()
            .aria_label(summary(row))
            .flex_wrap()
            .gap_2()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(border)
            .children(
                row.slug
                    .clone()
                    .map(|slug| text(cell("slug"), slug).text_sm()),
            )
            .children(
                row.title
                    .clone()
                    .map(|title| text(cell("title"), title).text_sm()),
            )
            .children(row.lifecycle.map(|lifecycle| {
                text(cell("lifecycle"), lifecycle_label(lifecycle))
                    .text_xs()
                    .text_color(muted)
            }))
            .child(
                text(cell("resolution"), resolution_label(row.resolved))
                    .text_xs()
                    .text_color(muted),
            )
            .child(text(cell("id"), row.id.clone()).text_xs().text_color(muted));
        match row.target(&self.repo().to_owned()) {
            Some(target) => {
                let focus = self
                    .row_focus
                    .entry((group.row, ix))
                    // A handle of its own is a tab stop only when it says so.
                    .or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                if focus.is_focused(window) {
                    element = element.border_2().border_color(ring);
                }
                element
                    .role(Role::Button)
                    .track_focus(&focus)
                    .cursor_pointer()
                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                        cx.emit(OpenRelated(target.clone()))
                    }))
                    .into_any_element()
            }
            None => element
                .child(
                    text(cell("cannot-open"), CANNOT_OPEN)
                        .text_xs()
                        .text_color(muted),
                )
                .into_any_element(),
        }
    }

    /// The ticket's problems with its `deps` and `parent`, each with its
    /// guidance, the ID it is about and its path.
    fn relationship_problems(
        &self,
        problems: &[ProblemDto],
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        if problems.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        Some(
            v_flex()
                .id("rel-problems")
                .test_support()
                .aria_label(PROBLEMS)
                .gap_1()
                .child(div().text_xs().text_color(muted).child(PROBLEMS))
                .children(problems.iter().enumerate().map(|(ix, problem)| {
                    v_flex()
                        .child(text(("rel-problem", ix), problem.code.as_str()).text_sm())
                        .child(
                            text(("rel-problem-guidance", ix), problem.guidance())
                                .text_xs()
                                .text_color(muted),
                        )
                        .children(problem.target_id.clone().map(|id| {
                            text(("rel-problem-target", ix), id)
                                .text_xs()
                                .text_color(muted)
                        }))
                        .children(problem.path.clone().map(|path| {
                            text(("rel-problem-path", ix), path)
                                .text_xs()
                                .text_color(muted)
                        }))
                })),
        )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "relationships_tests.rs"]
mod tests;
