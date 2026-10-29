//! Stamps: which view asked for a result, and whether it still wants it.
//!
//! Each model keeps a generation per scope and moves it on whenever the
//! scope's inputs change. A completion carries the stamp it was asked
//! under; `apply` drops it when the stamp is no longer current, so an old
//! result never replaces a newer view.

use std::collections::HashMap;

/// A registration root as the repository list returned it.
pub(crate) type RepoKey = String;

/// An open tab, by the number it was opened under.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) struct TabId(pub u64);

/// A page of the settings window.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum SettingsPage {
    Repositories,
    Remotes,
    Keys,
    Hosts,
}

/// What asked for a result.
///
/// Not `Copy`: a repository's scope holds its root.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ViewScope {
    Repositories,
    /// The selected repository's resolution. One scope for every
    /// repository: selecting another drops the last one's resolution.
    Selection,
    Documents(RepoKey),
    Tickets(RepoKey),
    Tab(TabId),
    /// The Find input's search. One scope: a new search drops the last.
    Find,
    Settings(SettingsPage),
    Operations(RepoKey),
    /// A confirmation dialog's preview. Each dialog keeps its own
    /// generations, so one scope serves them all.
    Confirmation,
    /// A create dialog's path check and new ID. Each dialog keeps its own
    /// generations, so one scope serves them all.
    Create,
    /// The draft registry's listing of the store. One scope: a new listing
    /// drops the last.
    Drafts,
    /// One Drafts page row's reads of its item. A new listing, or Retry,
    /// moves it on, so a late read never lands on a newer row.
    DraftRow(manyhands::drafts::DraftId),
}

/// A scope and the generation it was at when the request was made.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Stamp {
    pub scope: ViewScope,
    pub generation: u64,
}

/// The current generation of each scope a model has asked under.
#[derive(Default)]
pub(crate) struct Generations {
    current: HashMap<ViewScope, u64>,
}

impl Generations {
    /// Moves `scope` on and returns the stamp for a request made now.
    /// Every stamp issued for `scope` before this one stops being current.
    pub(crate) fn next(&mut self, scope: ViewScope) -> Stamp {
        let generation = self.current.entry(scope.clone()).or_insert(0);
        *generation += 1;
        Stamp {
            scope,
            generation: *generation,
        }
    }

    /// Whether `stamp` is the last one issued for its scope.
    pub(crate) fn is_current(&self, stamp: &Stamp) -> bool {
        self.current.get(&stamp.scope) == Some(&stamp.generation)
    }
}

#[cfg(test)]
#[path = "stamp_tests.rs"]
mod tests;
