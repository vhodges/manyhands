//! The repository selector: the list in the sidebar, and the selected
//! repository's detail in the content region.
//!
//! Rows are `list_repositories` rows, labeled with the root's directory
//! name and the full root as secondary text. Text is fixed per state or
//! `ResultCode::message()`; no error is ever formatted into it.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder,
};
use manyhands::repository::{ConfigurationDto, ConfigurationState};

use super::workspace::Workspace;
use crate::desktop::model::{
    list_state::{ListState, index_label},
    navigation::{Navigation, RepositoryEntry, Resolution},
};

pub(crate) const REPOSITORIES_HEADING: &str = "Repositories";
pub(crate) const UNAVAILABLE: &str = "Unavailable";
pub(crate) const RESOLVING: &str = "Opening";
pub(crate) const NOTHING_SELECTED: &str = "Select a repository.";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const REMOVE: &str = "Remove";

/// A configuration state in fixed text.
pub(crate) fn configuration_label(state: ConfigurationState) -> &'static str {
    match state {
        ConfigurationState::Valid => "Configuration valid",
        ConfigurationState::Invalid => "Configuration invalid",
        ConfigurationState::Missing => "Configuration missing",
    }
}

/// The repository list.
pub(crate) fn repositories(
    navigation: &Navigation,
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
    let selected = navigation.selected();
    let rows: Vec<_> = list
        .rows()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, entry)| row(index, entry, selected == Some(entry.key()), cx))
        .collect();
    v_flex()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_weight(gpui_kit::FontWeight::BOLD)
                .child(REPOSITORIES_HEADING),
        )
        .children(rows)
        .children(state)
        .children(failure)
}

fn row(
    index: usize,
    entry: &RepositoryEntry,
    selected: bool,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let (muted, accent) = (theme.muted_foreground, theme.accent);
    let key = entry.key().clone();
    let name = entry.name();
    let unavailable = matches!(entry.resolution, Resolution::Unavailable(_)).then(|| {
        div()
            .id(("repo-unavailable", index))
            .test_support()
            .aria_label(UNAVAILABLE)
            .text_xs()
            .child(UNAVAILABLE)
    });
    div()
        .id(("repo-row", index))
        .test_support()
        .role(Role::ListItem)
        .aria_label(name.clone())
        .aria_selected(selected)
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .when(selected, |row| row.bg(accent))
        .on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| this.select_repository(key.clone(), cx)),
        )
        .child(
            h_flex()
                .gap_2()
                .child(div().text_sm().child(name))
                .children(unavailable),
        )
        .child(
            div()
                .id(("repo-root", index))
                .test_support()
                .aria_label(entry.summary.root.clone())
                .text_xs()
                .text_color(muted)
                .child(entry.summary.root.clone()),
        )
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
