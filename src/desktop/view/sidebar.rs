//! The repository selector: the list in the sidebar, and the selected
//! repository's detail in the content region.
//!
//! Rows are `list_repositories` rows in the Kit's list, with its arrow and
//! Enter bindings, labeled with the root's directory name and the full
//! root as secondary text. The selected repository's row says Selected,
//! so it is not shown by color alone. Text is fixed per state or
//! `ResultCode::message()`; no error is ever formatted into it.

use gpui_kit::{
    App, AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, IndexPath,
        button::{Button, ButtonVariants},
        h_flex,
        list::{List, ListDelegate, ListItem, ListState as KitListState},
        v_flex,
    },
    div,
};
use manyhands::repository::{ConfigurationDto, ConfigurationState};

use super::workspace::Workspace;
use crate::desktop::model::{
    list_state::{ListState, index_label},
    navigation::{Navigation, RepositoryEntry, Resolution},
    stamp::RepoKey,
};

pub(crate) const REPOSITORIES_HEADING: &str = "Repositories";
pub(crate) const UNAVAILABLE: &str = "Unavailable";
pub(crate) const RESOLVING: &str = "Opening";
pub(crate) const NOTHING_SELECTED: &str = "Select a repository.";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const REMOVE: &str = "Remove";
/// The selected repository's row says so in text.
pub(crate) const SELECTED: &str = "Selected";

/// A configuration state in fixed text.
pub(crate) fn configuration_label(state: ConfigurationState) -> &'static str {
    match state {
        ConfigurationState::Valid => "Configuration valid",
        ConfigurationState::Invalid => "Configuration invalid",
        ConfigurationState::Missing => "Configuration missing",
    }
}

/// The repository list: `pane` shows its rows.
pub(crate) fn repositories(
    navigation: &Navigation,
    pane: &RepositoriesPane,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let muted = cx.theme().muted_foreground;
    let list = navigation.repositories();
    let state = match list {
        ListState::Ready { rows, complete, .. } if !rows.is_empty() && *complete => None,
        _ => Some(
            div()
                .id("repositories-state")
                .test_support()
                .aria_label(list.label())
                .text_sm()
                .text_color(muted)
                .child(list.label()),
        ),
    };
    let failure = match list {
        ListState::Busy | ListState::Failed(_) => Some(
            v_flex()
                .gap_1()
                .children(match list {
                    ListState::Failed(failure) => Some(
                        div()
                            .id("repositories-failure")
                            .test_support()
                            .aria_label(failure.code.message())
                            .text_sm()
                            .child(failure.code.message()),
                    ),
                    _ => None,
                })
                .child(
                    div().id("repositories-retry").test_support().child(
                        Button::new("repositories-retry-button")
                            .label(RETRY)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.load_repositories(cx)
                            })),
                    ),
                ),
        ),
        _ => None,
    };
    v_flex()
        .gap_1()
        .size_full()
        .child(
            div()
                .text_sm()
                .font_weight(gpui_kit::FontWeight::BOLD)
                .child(REPOSITORIES_HEADING),
        )
        .child(pane.render())
        .children(state)
        .children(failure)
}

/// One row of the selector, as drawn.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct RepositoryRow {
    pub key: RepoKey,
    name: String,
    unavailable: bool,
    /// The selected repository.
    selected: bool,
}

impl RepositoryRow {
    /// The rows `navigation` lists, in list order.
    pub(crate) fn rows(navigation: &Navigation) -> Vec<Self> {
        let selected = navigation.selected();
        navigation
            .repositories()
            .rows()
            .into_iter()
            .flatten()
            .map(|entry| Self::new(entry, selected == Some(entry.key())))
            .collect()
    }

    fn new(entry: &RepositoryEntry, selected: bool) -> Self {
        Self {
            key: entry.key().clone(),
            name: entry.name(),
            unavailable: matches!(entry.resolution, Resolution::Unavailable(_)),
            selected,
        }
    }
}

/// The Kit list's delegate: the rows, and the row the arrows are on,
/// which is not the selected repository until Enter or a click selects
/// it.
pub(crate) struct RepositoryDelegate {
    rows: Vec<RepositoryRow>,
    highlighted: Option<IndexPath>,
}

impl RepositoryDelegate {
    pub(crate) fn row(&self, ix: IndexPath) -> Option<&RepositoryRow> {
        self.rows.get(ix.row)
    }
}

impl ListDelegate for RepositoryDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<KitListState<Self>>,
    ) -> Option<ListItem> {
        let row = self.rows.get(ix.row)?;
        let index = ix.row;
        let muted = cx.theme().muted_foreground;
        let unavailable = row.unavailable.then(|| {
            div()
                .id(("repo-unavailable", index))
                .test_support()
                .aria_label(UNAVAILABLE)
                .text_xs()
                .child(UNAVAILABLE)
        });
        let selected = row.selected.then(|| {
            div()
                .id(("repo-selected", index))
                .test_support()
                .aria_label(SELECTED)
                .text_xs()
                .font_weight(gpui_kit::FontWeight::BOLD)
                .child(SELECTED)
        });
        Some(
            ListItem::new(("repo-item", index)).child(
                v_flex()
                    .id(("repo-row", index))
                    .test_support()
                    .aria_label(row.name.clone())
                    .aria_selected(row.selected)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().text_sm().child(row.name.clone()))
                            .children(selected)
                            .children(unavailable),
                    )
                    .child(
                        div()
                            .id(("repo-root", index))
                            .test_support()
                            .aria_label(row.key.clone())
                            .text_xs()
                            .text_color(muted)
                            .child(row.key.clone()),
                    ),
            ),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<KitListState<Self>>,
    ) {
        self.highlighted = ix;
        cx.notify();
    }
}

/// The selector's Kit list. A click or Enter on a row emits the Kit's
/// `ListEvent::Confirm`, which the workspace answers by selecting it.
pub(crate) struct RepositoriesPane {
    list: Entity<KitListState<RepositoryDelegate>>,
}

impl RepositoriesPane {
    pub(crate) fn new(window: &mut Window, cx: &mut App) -> Self {
        let delegate = RepositoryDelegate {
            rows: Vec::new(),
            highlighted: None,
        };
        Self {
            list: cx.new(|cx| KitListState::new(delegate, window, cx)),
        }
    }

    pub(crate) fn list(&self) -> &Entity<KitListState<RepositoryDelegate>> {
        &self.list
    }

    /// Shows `rows` when they differ from those shown. The row the arrows
    /// are on stays where the new rows still have it; with none, they
    /// start from the selected repository.
    pub(crate) fn set(&self, rows: Vec<RepositoryRow>, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            if list.delegate().rows == rows {
                return;
            }
            let highlighted = list
                .selected_index()
                .and_then(|ix| list.delegate().rows.get(ix.row))
                .map(|row| row.key.clone());
            let at = highlighted
                .and_then(|key| rows.iter().position(|row| row.key == key))
                .or_else(|| rows.iter().position(|row| row.selected));
            list.delegate_mut().rows = rows;
            list.set_selected_index(at.map(IndexPath::new), window, cx);
            cx.notify();
        });
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| list.focus(window, cx));
    }

    fn render(&self) -> impl IntoElement + use<> {
        div()
            .id("repositories-list")
            .test_support()
            .flex_1()
            .min_h_0()
            .child(List::new(&self.list))
    }
}

/// The selected repository: its configuration and index, or why it is
/// unavailable with Retry and Remove.
pub(crate) fn selection(
    navigation: &Navigation,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let Some(entry) = navigation.selected_entry() else {
        return v_flex()
            .gap_2()
            .child(div().text_sm().child(NOTHING_SELECTED));
    };
    let muted = cx.theme().muted_foreground;
    let key = entry.key().clone();
    let heading = v_flex()
        .child(
            div()
                .id("selection-name")
                .test_support()
                .aria_label(entry.name())
                .text_lg()
                .child(entry.name()),
        )
        .child(
            div()
                .text_xs()
                .text_color(muted)
                .child(entry.summary.root.clone()),
        );
    let body = match &entry.resolution {
        Resolution::Unknown => v_flex().child(
            div()
                .id("selection-resolving")
                .test_support()
                .aria_label(RESOLVING)
                .child(RESOLVING),
        ),
        Resolution::Resolved(_) => resolved(entry, cx),
        Resolution::Unavailable(code) => v_flex()
            .gap_2()
            .child(
                div()
                    .id("selection-unavailable-state")
                    .test_support()
                    .aria_label(UNAVAILABLE)
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(UNAVAILABLE),
            )
            .child(
                div()
                    .id("selection-unavailable")
                    .test_support()
                    .aria_label(code.message())
                    .child(code.message()),
            )
            .child(div().text_xs().text_color(muted).child(code.as_str()))
            .child(
                h_flex().gap_2().child(retry(key, cx)).child(
                    // Task 13 prepares `RepoRemove` for this root; until
                    // then the control is shown and does nothing.
                    div().id("selection-remove").test_support().child(
                        Button::new("selection-remove-button")
                            .danger()
                            .label(REMOVE)
                            .disabled(true),
                    ),
                ),
            ),
        Resolution::Busy => v_flex()
            .gap_2()
            .child(
                div()
                    .id("selection-busy")
                    .test_support()
                    .aria_label(crate::desktop::model::list_state::LABEL_BUSY)
                    .child(crate::desktop::model::list_state::LABEL_BUSY),
            )
            .child(retry(key, cx)),
        Resolution::Failed(failure) => v_flex()
            .gap_2()
            .child(
                div()
                    .id("selection-failed")
                    .test_support()
                    .aria_label(failure.code.message())
                    .child(failure.code.message()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(failure.code.as_str()),
            )
            .child(retry(key, cx)),
    };
    v_flex().gap_3().child(heading).child(body)
}

fn retry(key: String, cx: &mut Context<Workspace>) -> impl IntoElement + use<> {
    div().id("selection-retry").test_support().child(
        Button::new("selection-retry-button").label(RETRY).on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| this.retry_resolution(key.clone(), cx)),
        ),
    )
}

/// The configuration and index state the list read stored for a
/// resolved entry, with an invalid configuration's problems.
fn resolved(entry: &RepositoryEntry, cx: &mut Context<Workspace>) -> gpui_kit::Div {
    let muted = cx.theme().muted_foreground;
    let ConfigurationDto {
        state,
        primary_branch,
        problems,
        ..
    } = &entry.summary.configuration;
    let index = index_label(&entry.summary.index);
    v_flex()
        .gap_1()
        .child(
            div()
                .id("selection-configuration")
                .test_support()
                .aria_label(configuration_label(*state))
                .child(configuration_label(*state)),
        )
        .children(primary_branch.clone().map(|branch| {
            h_flex()
                .gap_1()
                .text_sm()
                .child(div().text_color(muted).child("Primary branch"))
                .child(branch)
        }))
        .children(problems.iter().enumerate().map(|(index, problem)| {
            div()
                .id(("selection-problem", index))
                .test_support()
                .aria_label(problem.code.guidance())
                .text_sm()
                .child(problem.code.guidance())
        }))
        .child(
            div()
                .id("selection-index")
                .test_support()
                .aria_label(index)
                .child(index),
        )
}
