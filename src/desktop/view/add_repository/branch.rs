//! Add's main branch (Wave 04 Cycle 05a design B7; R-12; decision 10): a
//! Kit select (`add-branch`) whose items are made when the check lands: the
//! checked-out branch first, chosen and tagged Suggested, also when it has
//! no commits yet and so is not among the local branches (fact 11a); then
//! every other local branch, greyed with why. With a configuration on
//! another main branch, nothing can be chosen, and B6's row says why.
//!
//! A Kit select's disabled item is only drawn disabled: a click, Enter or
//! the arrows still reach it (fact 38). So the delegate refuses it in
//! `on_will_change`, and a `Confirm` naming a branch that cannot be chosen
//! is ignored: the select is set back.

use gpui_kit::{
    App, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
    component::{
        ActiveTheme, Icon, IndexPath, h_flex,
        searchable_list::SearchableListChange,
        select::{Select, SelectDelegate, SelectEvent, SelectItem},
        tooltip::Tooltip,
        v_flex,
    },
    div,
    prelude::FluentBuilder,
    px,
};

use super::{
    AddRepositorySheet, CHECK_OUT_FIRST, Form, MAIN_BRANCH, MAIN_BRANCH_HELP, MAIN_BRANCH_TIP,
    SUGGESTED,
    checks::{Inspection, allowed_branch},
};
use crate::desktop::{
    assets::icon_path,
    view::{
        settings::{field_column, wrapped},
        shell::tokens::chrome_colors,
    },
};

/// What a branch is to the add.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BranchKind {
    /// Checked out, and the one the add starts from: chosen.
    Suggested,
    /// Not checked out: "Check it out first."
    Other,
    /// Checked out, but the configuration names another main branch.
    Held,
}

/// One item of the select.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Branch {
    pub(super) name: String,
    pub(super) kind: BranchKind,
}

impl SelectItem for Branch {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &String {
        &self.name
    }

    fn disabled(&self) -> bool {
        self.kind != BranchKind::Suggested
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = chrome_colors(cx);
        let name = h_flex().gap(px(6.)).child(self.name.clone()).when(
            self.kind == BranchKind::Suggested,
            |name| {
                name.child(
                    div()
                        .text_size(px(11.))
                        .text_color(colors.teal)
                        .child(SUGGESTED),
                )
            },
        );
        v_flex()
            .child(name)
            .when(self.kind == BranchKind::Other, |item| {
                item.child(
                    div()
                        .whitespace_normal()
                        .text_size(px(11.))
                        .text_color(colors.muted)
                        .child(CHECK_OUT_FIRST),
                )
            })
    }
}

/// The select's items, which refuse every branch but the one the add
/// starts from (R-12).
#[derive(Default)]
pub(crate) struct Branches(Vec<Branch>);

impl Branches {
    pub(super) fn new(items: Vec<Branch>) -> Self {
        Self(items)
    }
}

impl SelectDelegate for Branches {
    type Item = Branch;

    fn items_count(&self, _: usize) -> usize {
        self.0.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Branch> {
        self.0.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Branch: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.0
            .iter()
            .position(|branch| branch.value() == value)
            .map(|ix| IndexPath::default().row(ix))
    }

    /// A single choice: the branch selected last replaces the selection,
    /// unless it cannot be chosen, when the selection stays as it was.
    fn on_will_change(
        &mut self,
        selection: &mut Vec<(IndexPath, Branch)>,
        changes: &[SearchableListChange],
    ) {
        let chosen = changes.iter().rev().find_map(|change| match change {
            SearchableListChange::Select { index } => Some(*index),
            SearchableListChange::Deselect { .. } => None,
        });
        let Some(index) = chosen else {
            return;
        };
        if let Some(branch) = self.item(index).filter(|branch| !branch.disabled()) {
            *selection = vec![(index, branch.clone())];
        }
    }
}

/// The items a check gives the select, and which is chosen.
fn items(inspection: &Inspection) -> (Vec<Branch>, Option<usize>) {
    let Inspection::Checked { inspection, .. } = inspection else {
        return (Vec::new(), None);
    };
    let allowed = allowed_branch(inspection);
    let head = inspection.head_branch.as_ref();
    let mut items: Vec<Branch> = head
        .map(|name| Branch {
            name: name.clone(),
            kind: if allowed.is_some() {
                BranchKind::Suggested
            } else {
                BranchKind::Held
            },
        })
        .into_iter()
        .collect();
    items.extend(
        inspection
            .local_branches
            .iter()
            .filter(|name| Some(*name) != head)
            .map(|name| Branch {
                name: name.clone(),
                kind: BranchKind::Other,
            }),
    );
    (items, allowed.map(|_| 0))
}

impl AddRepositorySheet {
    /// Gives the select the items of the check now, once the check changed.
    pub(super) fn sync_branches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.branches_stale) {
            return;
        }
        let (items, chosen) = items(&self.inspection);
        self.branches = items.clone();
        self.branch.update(cx, |select, cx| {
            select.set_items(Branches::new(items), window, cx);
            select.set_selected_index(chosen.map(|ix| IndexPath::default().row(ix)), window, cx);
        });
    }

    /// A `Confirm` that names a branch other than the one the add starts
    /// from is ignored: the select goes back to it (R-12).
    pub(super) fn branch_confirmed(
        &mut self,
        event: &SelectEvent<Branches>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(named) = event;
        let allowed = match &self.inspection {
            Inspection::Checked { inspection, .. } => allowed_branch(inspection),
            _ => None,
        };
        if named.as_deref() != allowed {
            self.branches_stale = true;
            self.sync_branches(window, cx);
            cx.notify();
        }
    }

    /// The Main branch field: its label, with its tooltip, the select in
    /// its wrapper, and its help (B7).
    pub(super) fn branch_field(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let id = Form::Add.branch();
        let select = h_flex()
            .gap(px(6.))
            .items_center()
            .child(
                Icon::default()
                    .path(icon_path("git-branch"))
                    .text_color(colors.secondary)
                    .w(px(14.))
                    .h(px(14.)),
            )
            .child(
                div().flex_1().child(
                    Select::new(&self.branch)
                        .id("add-branch-select")
                        .accessibility_label(MAIN_BRANCH)
                        .placeholder(""),
                ),
            );
        field_column()
            .gap_0p5()
            .child(
                div()
                    .id("add-branch-label")
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(MAIN_BRANCH)
                    .tooltip(|window, cx| Tooltip::new(MAIN_BRANCH_TIP).build(window, cx)),
            )
            .child(wrapped(id, MAIN_BRANCH, &self.focus[id], &self.scroll, select).w_full())
            .child(
                div()
                    .text_xs()
                    .text_color(colors.secondary)
                    .child(MAIN_BRANCH_HELP),
            )
    }

    /// The branch the select has chosen (P10; E-1b).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn chosen_branch(&self, cx: &App) -> Option<String> {
        self.branch.read(cx).selected_value().cloned()
    }

    /// The items the select was given last (P10).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn branch_items(&self) -> &[Branch] {
        &self.branches
    }

    /// The select's own state (P10's keyboard and `Confirm` rows).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn branch_select(
        &self,
    ) -> gpui_kit::Entity<gpui_kit::component::select::SelectState<Branches>> {
        self.branch.clone()
    }
}
