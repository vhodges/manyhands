//! The open tabs: an ordered list of keys and the active one.
//!
//! A tab is keyed by its repository and what it shows: an item by kind
//! and ID, or a nonconforming entry by worktree and path. Never by short
//! code, which two repositories, or two items, can share; and the
//! repository is part of the key, since two repositories can hold the
//! same ID.

use manyhands::repository::ItemDtoKind;

use super::{
    navigation::OpenTarget,
    stamp::{RepoKey, TabId},
};

/// What a tab shows.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum TabKey {
    Item {
        repo: RepoKey,
        kind: ItemDtoKind,
        id: String,
    },
    Inspect {
        repo: RepoKey,
        worktree: String,
        path: String,
    },
}

impl TabKey {
    /// The tab `target` opens in.
    pub(crate) fn for_target(target: &OpenTarget) -> Self {
        match target {
            OpenTarget::Item { repo, kind, id } => Self::Item {
                repo: repo.clone(),
                kind: *kind,
                id: id.to_string(),
            },
            OpenTarget::Inspect {
                repo,
                worktree,
                path,
            } => Self::Inspect {
                repo: repo.clone(),
                worktree: worktree.clone(),
                path: path.clone(),
            },
        }
    }

    /// The registration root the tab reads from.
    pub(crate) fn repo(&self) -> &RepoKey {
        match self {
            Self::Item { repo, .. } | Self::Inspect { repo, .. } => repo,
        }
    }
}

/// The open tabs in strip order, and the one shown.
#[derive(Default)]
pub(crate) struct Tabs {
    open: Vec<(TabId, TabKey)>,
    active: Option<TabId>,
    /// The last ID given; IDs are never reused, so a stamp for a closed
    /// tab can never match a new one.
    last_id: u64,
}

impl Tabs {
    /// The open tabs, in strip order.
    pub(crate) fn tabs(&self) -> &[(TabId, TabKey)] {
        &self.open
    }

    pub(crate) fn active(&self) -> Option<TabId> {
        self.active
    }

    #[cfg(test)]
    pub(crate) fn key(&self, id: TabId) -> Option<&TabKey> {
        self.open
            .iter()
            .find(|(open, _)| *open == id)
            .map(|(_, key)| key)
    }

    fn position(&self, id: TabId) -> Option<usize> {
        self.open.iter().position(|(open, _)| *open == id)
    }

    /// Shows `key`'s tab: the one already open, and false, or a new one at
    /// the end, and true.
    pub(crate) fn open(&mut self, key: TabKey) -> (TabId, bool) {
        if let Some((id, _)) = self.open.iter().find(|(_, open)| *open == key) {
            let id = *id;
            self.active = Some(id);
            return (id, false);
        }
        self.last_id += 1;
        let id = TabId(self.last_id);
        self.open.push((id, key));
        self.active = Some(id);
        (id, true)
    }

    /// The open tab `id` shows `key` from now on, in its place in the strip:
    /// the tab follows its item (a repair made). False when it is not open,
    /// or another tab shows `key`.
    pub(crate) fn replace(&mut self, id: TabId, key: TabKey) -> bool {
        if self
            .open
            .iter()
            .any(|(open, shown)| *open != id && *shown == key)
        {
            return false;
        }
        match self.open.iter_mut().find(|(open, _)| *open == id) {
            Some((_, shown)) => {
                *shown = key;
                true
            }
            None => false,
        }
    }

    /// Shows the open tab `id`. False when it is not open.
    pub(crate) fn activate(&mut self, id: TabId) -> bool {
        if self.position(id).is_none() {
            return false;
        }
        self.active = Some(id);
        true
    }

    /// Closes `id`. When it was shown, the tab that takes its place in the
    /// strip is, or the one before it when it was the last; with none
    /// left, nothing is. False when it was not open.
    pub(crate) fn close(&mut self, id: TabId) -> bool {
        let Some(at) = self.position(id) else {
            return false;
        };
        self.open.remove(at);
        if self.active == Some(id) {
            self.active = self
                .open
                .get(at)
                .or_else(|| at.checked_sub(1).and_then(|before| self.open.get(before)))
                .map(|(id, _)| *id);
        }
        true
    }

    /// Shows the tab `step` places from the shown one, wrapping.
    fn step(&mut self, step: isize) -> Option<TabId> {
        let count = self.open.len() as isize;
        if count == 0 {
            return None;
        }
        let at = match self.active.and_then(|id| self.position(id)) {
            Some(at) => (at as isize + step).rem_euclid(count) as usize,
            None => 0,
        };
        let id = self.open[at].0;
        self.active = Some(id);
        Some(id)
    }

    /// Shows the next tab, wrapping to the first.
    pub(crate) fn next(&mut self) -> Option<TabId> {
        self.step(1)
    }

    /// Shows the previous tab, wrapping to the last.
    pub(crate) fn previous(&mut self) -> Option<TabId> {
        self.step(-1)
    }

    /// The open tabs of `root`, in strip order.
    pub(crate) fn of_repository(&self, root: &str) -> Vec<TabId> {
        self.open
            .iter()
            .filter(|(_, key)| key.repo() == root)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Closes every tab of `root`, as its registration's removal does, and
    /// returns them; a later registration at the same root opens new ones.
    pub(crate) fn close_repository(&mut self, root: &str) -> Vec<TabId> {
        let closed = self.of_repository(root);
        for id in &closed {
            self.close(*id);
        }
        closed
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
