//! A repository's menu (Part D): which items it shows, in which order, and
//! which are disabled, from the same facts and state as the row. The view
//! draws what this says; a choice is checked again, by evaluating this
//! again, when it is chosen (R-35).

mod check_all;

pub(crate) use self::check_all::check_all_applies;
use super::{
    super::{
        list_view::{REBUILD_INDEX, REFRESH},
        polling::status::{Controls, Shape},
        stamp::RepoKey,
    },
    RepoFacts, RepoState, RowAction, StateKind, action_label, sentence_line,
};
use crate::desktop::view::sidebar::{SHARE_MAIN, SHARE_MAIN_SET_UP, SHARING};

pub(crate) const CHECK_NOW: &str = "Check now";
pub(crate) const RETRY_NOW: &str = "Retry now";
pub(crate) const SET_UP_SHARING: &str = "Set up sharing\u{2026}";
pub(crate) const SETTINGS: &str = "Settings\u{2026}";
pub(crate) const OPEN_FOLDER: &str = "Open folder";
pub(crate) const REMOVE_FROM_MANYHANDS: &str = "Remove from Manyhands\u{2026}";

/// What an item of a repository's menu, or of More, does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MenuChoice {
    /// The state's sentence: read, never chosen.
    Sentence,
    /// The state's own action (`RepoState::action`).
    StateAction,
    CheckNow,
    ShareMain,
    OpenFolder,
    Settings,
    Remove,
    Refresh,
    Rebuild,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct MenuItem {
    pub choice: MenuChoice,
    pub label: String,
    pub enabled: bool,
}

impl MenuItem {
    fn new(choice: MenuChoice, label: &str, enabled: bool) -> Self {
        Self {
            choice,
            label: label.to_owned(),
            enabled,
        }
    }
}

/// The items of a repository's menu, in order: the state's sentence first
/// and disabled, the state's action, Check now (or Retry now) unless that
/// is the state's action, Share main, Open folder, Settings… unless that is
/// the state's action, and Remove from Manyhands….
pub(crate) fn menu_items(facts: &RepoFacts<'_>, state: &RepoState) -> Vec<MenuItem> {
    use MenuChoice::*;
    let mut items = Vec::new();
    let sentence = sentence_line(state);
    if !sentence.is_empty() {
        items.push(MenuItem::new(Sentence, &sentence, false));
    }
    let row = facts.row();
    let polling = match &row.shape {
        Shape::Polling(controls) => Some(*controls),
        _ => None,
    };
    // D6's shape polls and nothing is checking.
    let can_check = polling.is_some() && !facts.checking();
    if let Some(action) = &state.action {
        let enabled = match action {
            RowAction::Unlock(_) => facts.unlock_target,
            RowAction::CheckNow | RowAction::RetryNow => can_check,
            _ => true,
        };
        items.push(MenuItem::new(StateAction, action_label(action), enabled));
    }
    if !matches!(
        state.action,
        Some(RowAction::CheckNow | RowAction::RetryNow)
    ) {
        let retry = matches!(polling, Some(Controls { retry: true, .. }));
        let label = if retry { RETRY_NOW } else { CHECK_NOW };
        items.push(MenuItem::new(CheckNow, label, can_check));
    }
    let cant_open = state.kind == StateKind::CantOpen;
    let sharing = if facts.sharing_main {
        SHARING
    } else if facts
        .entry
        .summary
        .configuration
        .publication_remote
        .is_some()
    {
        SHARE_MAIN
    } else {
        SHARE_MAIN_SET_UP
    };
    let not_ssh = state.kind == StateKind::NotSsh;
    items.push(MenuItem::new(
        ShareMain,
        sharing,
        !(facts.sharing_main || cant_open || not_ssh),
    ));
    items.push(MenuItem::new(OpenFolder, OPEN_FOLDER, !cant_open));
    if state.action != Some(RowAction::Settings) {
        items.push(MenuItem::new(Settings, SETTINGS, true));
    }
    items.push(MenuItem::new(Remove, REMOVE_FROM_MANYHANDS, true));
    items
}

/// The More menu's items: Refresh, Rebuild the search index and Remove from
/// Manyhands…, each disabled while no repository is chosen.
pub(crate) fn more_items(chosen: Option<&RepoKey>) -> Vec<MenuItem> {
    use MenuChoice::*;
    let enabled = chosen.is_some();
    vec![
        MenuItem::new(Refresh, REFRESH, enabled),
        MenuItem::new(Rebuild, REBUILD_INDEX, enabled),
        MenuItem::new(Remove, REMOVE_FROM_MANYHANDS, enabled),
    ]
}
