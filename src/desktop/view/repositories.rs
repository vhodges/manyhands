//! The Repositories panel (Cycle 03a, Part C): one row per repository the
//! list gives, plus the chosen one it no longer gives (the kept row), each
//! with its state from `repository_state`; and the states of the list read.
//!
//! The Workspace computes the rows, since the panel's body is built while
//! the Workspace is being updated and cannot read it; it pushes them to the
//! pane after the frame (R-8). The rows are the panel's own element, not
//! `ListRow` (R-7), drawn in `row`; the list's states in `body`.

mod body;
mod row;

use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use gpui_kit::{
    App, AppContext, Bounds, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    Styled, TestSupportExt, WeakEntity, Window,
    component::{
        IndexPath,
        list::{List, ListState as KitListState},
    },
    div,
};

pub(crate) use self::body::body;
use self::row::RepositoryDelegate;
use super::workspace::Workspace;
use crate::desktop::{
    model::{
        navigation::RepositoryEntry,
        polling::{Phase, Reason},
        repository_state::{
            MenuItem, RepoFacts, RepoState, menu_items, repository_state, repository_tooltip,
        },
        stamp::RepoKey,
    },
    services::{Services, requests::Intent},
};

pub(crate) const LOADING_REPOSITORIES: &str = "Loading repositories";
pub(crate) const NO_REPOSITORIES: &str = "No repositories yet.";
/// Part E's Add, which the empty state offers too.
pub(crate) const ADD_OR_CREATE: &str = "Add or create a repository\u{2026}";
pub(crate) const NOT_UPDATED_BUSY: &str = "Repositories were not updated: Manyhands is busy.";
pub(crate) const LIST_INCOMPLETE: &str = "The list of repositories is incomplete.";

/// Where each row was drawn, by root, recorded during prepaint: the anchor
/// a key-opened menu takes (Part D).
pub(crate) type RowBounds = Rc<RefCell<HashMap<RepoKey, Bounds<Pixels>>>>;

/// One row of the panel, as drawn: what the Workspace knows of the
/// repository, put in words by `repository_state`.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct RepositoryRow {
    pub key: RepoKey,
    name: String,
    state: RepoState,
    /// `repository_tooltip`'s lines: the tooltip, and with " " the row's
    /// accessible description.
    tooltip: Vec<String>,
    menu: Vec<MenuItem>,
    /// The chosen repository: it carries the selected treatment and mark.
    chosen: bool,
    /// The chosen repository the list no longer gives (decision 22).
    kept: bool,
}

impl RepositoryRow {
    #[cfg(test)]
    #[allow(dead_code)] // Used from Task 9
    pub(crate) fn tooltip(&self) -> &[String] {
        &self.tooltip
    }

    #[cfg(test)]
    #[allow(dead_code)] // Used from Task 9
    pub(crate) fn menu(&self) -> &[MenuItem] {
        &self.menu
    }
}

/// The rows the panel draws: each listed repository in list order, then
/// the kept one (decision 22). Read from `Navigation`, the scheduler, the
/// worker's in-flight requests and the shares of main.
pub(crate) fn rows(workspace: &Workspace, cx: &App) -> Vec<RepositoryRow> {
    let navigation = workspace.navigation();
    let services = workspace.services();
    let scheduler = services.scheduler().read(cx);
    let operations = services.operations().read(cx);
    let chosen = navigation.selected();
    let polling = services.polling();
    let now = scheduler.utc();
    // Task 4 reads the titles of what is not shared yet.
    let titles = |_: &manyhands::canonical::ItemId| None;
    let listed = navigation
        .repositories()
        .rows()
        .into_iter()
        .flatten()
        .map(|entry| (entry, false));
    listed
        .chain(navigation.kept().map(|entry| (entry, true)))
        .map(|(entry, kept): (&RepositoryEntry, bool)| {
            let root = entry.key();
            let schedule = scheduler.schedule().repo(root);
            let unlock_target = match schedule.map(|schedule| &schedule.phase) {
                Some(Phase::Attention(Reason::KeyLocked(key))) => {
                    scheduler.unlock_target(*key, operations).is_some()
                }
                _ => false,
            };
            let facts = RepoFacts {
                entry,
                kept,
                schedule,
                unplaced: scheduler.unplaced(root),
                holding: scheduler.holding(root),
                stored: scheduler.stored(root),
                shared_copy: scheduler.shared_copy(root),
                checking_now: checking_now(services, Path::new(root)),
                sharing_main: workspace.sharing_main(root),
                polling,
                unlock_target,
                titles: &titles,
                now,
            };
            let state = repository_state(&facts);
            RepositoryRow {
                key: root.clone(),
                name: entry.name(),
                tooltip: repository_tooltip(&facts, &state),
                menu: menu_items(&facts, &state),
                state,
                chosen: chosen == Some(root),
                kept,
            }
        })
        .collect()
}

/// This process's own Check now of `root` is queued or running.
fn checking_now(services: &Services, root: &Path) -> bool {
    services
        .worker()
        .in_flight(|intent| matches!(intent, Intent::PollOnce { root: asked } if asked == root))
}

/// The panel's Kit list. A click or Enter on a row emits the Kit's
/// `ListEvent::Confirm`, which the Workspace answers by choosing it.
pub(crate) struct RepositoriesPane {
    list: Entity<KitListState<RepositoryDelegate>>,
    /// The rows' bounds, shared with the delegate, which records them.
    #[allow(dead_code)] // Used from Task 5
    bounds: RowBounds,
}

impl RepositoriesPane {
    /// The pane of the Workspace `workspace`, which a row's pointer hook
    /// reaches (R-36).
    pub(crate) fn new(workspace: WeakEntity<Workspace>, window: &mut Window, cx: &mut App) -> Self {
        let bounds = RowBounds::default();
        let delegate = RepositoryDelegate::new(workspace, bounds.clone());
        let list = cx.new(|cx| KitListState::new(delegate, window, cx));
        list.update(cx, |list, cx| {
            let focus = gpui_kit::Focusable::focus_handle(list, cx);
            list.delegate_mut().set_focus(focus);
        });
        Self { list, bounds }
    }

    pub(crate) fn list(&self) -> &Entity<KitListState<RepositoryDelegate>> {
        &self.list
    }

    /// The rows shown.
    pub(crate) fn rows(&self, cx: &App) -> Vec<RepositoryRow> {
        self.list.read(cx).delegate().rows().to_vec()
    }

    /// Shows `rows` after the frame being drawn, when they differ from
    /// those shown, so nothing is changed while the window draws (R-8).
    pub(crate) fn show_after_frame(
        &self,
        rows: Vec<RepositoryRow>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.rows(cx) == rows {
            return;
        }
        let list = self.list.clone();
        window.defer(cx, move |window, cx| show(&list, rows, window, cx));
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

/// Shows `rows`. The row the arrows are on stays where the new rows still
/// have it; with none, they start from the chosen repository.
fn show(
    list: &Entity<KitListState<RepositoryDelegate>>,
    rows: Vec<RepositoryRow>,
    window: &mut Window,
    cx: &mut App,
) {
    list.update(cx, |list, cx| {
        if list.delegate().rows() == rows.as_slice() {
            return;
        }
        let cursor = list
            .selected_index()
            .and_then(|ix| list.delegate().row(ix))
            .map(|row| row.key.clone());
        let at = cursor
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .or_else(|| rows.iter().position(|row| row.chosen));
        list.delegate_mut().set_rows(rows);
        list.set_selected_index(at.map(IndexPath::new), window, cx);
        cx.notify();
    });
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "repositories_tests.rs"]
mod tests;
