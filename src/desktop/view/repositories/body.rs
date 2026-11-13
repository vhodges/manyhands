//! The Repositories panel's body: the state of the repositories read above
//! the list, which is drawn in every state (C2 R-34), and the Settings
//! button below it until Cycle 04 (decision 15). No heading: the panel's
//! header names it (fact 2).
//!
//! The states are Part C's table, drawn by Cycle 02's helpers (R-10):
//! skeleton rows on the first load; an alert row when a read stayed busy
//! or failed, or the list is incomplete, over the rows kept; an empty
//! state when the list gives none. Retry reads the list again.

use gpui_kit::{
    App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        button::{Button, ButtonVariants},
        v_flex,
    },
    div,
};

use super::{
    ADD_OR_CREATE, LIST_INCOMPLETE, LOADING_REPOSITORIES, NO_REPOSITORIES, NOT_UPDATED_BUSY,
    RepositoriesPane,
};
use crate::desktop::{
    model::{
        list_state::ListState,
        list_view::{Alert, AlertTone, ListAction},
        navigation::{Navigation, NotReloaded},
        stamp::SettingsPage,
    },
    view::{
        rows::{EmptyAction, alert_row, empty_state, skeleton_rows},
        workspace::Workspace,
    },
};

/// The panel's entry to the settings.
const SETTINGS: &str = "Settings";

/// The skeleton's rows on a first load.
const SKELETON_ROWS: usize = 3;

/// What the body draws above the list.
#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    /// The first load.
    Loading,
    /// The alert row, over the rows when there are any.
    Alert(Alert),
    /// A list that gives no repository.
    Empty,
    /// Nothing: the rows say it all.
    Rows,
}

fn alert(tone: AlertTone, text: &str) -> State {
    State::Alert(Alert {
        tone,
        text: text.to_owned(),
        detail: None,
        action: ListAction::Retry,
    })
}

/// Part C's list states, in its table's order. A reload that stayed busy
/// or failed keeps the rows, and says so before an incomplete list does.
fn state(navigation: &Navigation) -> State {
    let busy = || alert(AlertTone::Warning, NOT_UPDATED_BUSY);
    let failed = |code: manyhands::results::ResultCode| alert(AlertTone::Failure, code.message());
    match navigation.repositories() {
        ListState::Loading => State::Loading,
        ListState::Busy => busy(),
        ListState::Failed(failure) => failed(failure.code),
        ListState::Ready { rows, complete, .. } => match navigation.not_reloaded() {
            Some(NotReloaded::Busy) => busy(),
            Some(NotReloaded::Failed(failure)) => failed(failure.code),
            None if !complete => alert(AlertTone::Warning, LIST_INCOMPLETE),
            None if rows.is_empty() && navigation.kept().is_none() => State::Empty,
            None => State::Rows,
        },
    }
}

/// The panel's body: the list's state, the list, and Settings.
pub(crate) fn body(
    navigation: &Navigation,
    pane: &RepositoriesPane,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let workspace = cx.entity().downgrade();
    let shown = match state(navigation) {
        State::Loading => Some(
            skeleton_rows(
                "repositories-loading",
                SKELETON_ROWS,
                LOADING_REPOSITORIES,
                cx,
            )
            .into_any_element(),
        ),
        State::Alert(alert) => {
            let retry = move |_: &mut Window, cx: &mut App| {
                let _ = workspace.update(cx, |workspace, cx| workspace.load_repositories(cx));
            };
            Some(alert_row("repositories-alert", &alert, retry, cx).into_any_element())
        }
        State::Empty => {
            let add: EmptyAction = (
                ADD_OR_CREATE.into(),
                Box::new(move |window: &mut Window, cx: &mut App| {
                    let _ = workspace.update(cx, |workspace, cx| {
                        workspace.open_settings(SettingsPage::Repositories, window, cx)
                    });
                }),
            );
            Some(
                empty_state("repositories-empty", NO_REPOSITORIES, Some(add), cx)
                    .into_any_element(),
            )
        }
        State::Rows => None,
    };
    v_flex()
        .gap_1()
        .size_full()
        .children(shown)
        .child(pane.render())
        .child(
            div()
                .id("sidebar-settings")
                .test_support()
                .aria_label(SETTINGS)
                .child(
                    Button::new("sidebar-settings-button")
                        .ghost()
                        .label(SETTINGS)
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.open_settings(SettingsPage::Repositories, window, cx)
                        })),
                ),
        )
}
