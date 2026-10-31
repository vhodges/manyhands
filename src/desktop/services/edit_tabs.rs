//! What each item tab of this window says of its edit session, for the
//! views that act on a tab's item from elsewhere (Part C fixes 1 and 2;
//! the Cycle's contract 6 and R-35):
//!
//! - **Unkept text.** A tab whose session holds unsaved text that no draft
//!   holds: crash recovery off, a draft that could not begin, or one still
//!   beginning. A repository's removal says how many of its tabs hold such
//!   text, and that removing closes them. The tabs it counted when the user
//!   pressed Confirm are the user's choice: when the removal lands, a tab
//!   that became unkept after it stays open (`confirmed`, `counted`).
//! - **Editing.** A tab that holds an edit session of its item, live or
//!   kept, or is getting one ready: the Drafts page does not restore a
//!   draft of that item into it, a second session over the first.
//!
//! Each item tab says so of itself, for as long as it lives (`watch`), as
//! it tells release on leave whether it edits. Nothing here holds text: a
//! tab is named by its key, a repository and an item ID.

use std::collections::{HashMap, HashSet};

use gpui_kit::{App, Context};

use crate::desktop::model::{stamp::RepoKey, tabs::TabKey};

/// What an item tab says of its edit session. It holds no text.
#[derive(Clone, Copy, Default)]
pub(crate) struct TabEditing {
    /// It holds an edit session of its item, live or kept, or is getting
    /// one ready.
    pub editing: bool,
    /// The session holds unsaved text that no draft holds.
    pub unkept: bool,
}

/// What an item tab says, as it is now; `None` once the tab is gone.
pub(crate) type Watch = Box<dyn Fn(&App) -> Option<TabEditing>>;

/// Every item tab of this window, and what each removal the user confirmed
/// counted.
#[derive(Default)]
pub(crate) struct EditTabs {
    tabs: Vec<(TabKey, Watch)>,
    /// The tabs of a repository whose text no draft held when its removal
    /// was confirmed, until that removal lands.
    counted: HashMap<RepoKey, HashSet<TabKey>>,
}

impl EditTabs {
    /// The tab `tab` says, through `watch`, what its edit session is.
    pub(crate) fn watch(&mut self, tab: TabKey, watch: Watch) {
        self.tabs.push((tab, watch));
    }

    /// The open tabs of `root` whose session holds unsaved text that no
    /// draft holds. A tab that is gone is forgotten. It is read as a dialog
    /// is drawn or pressed, when no tab is being updated.
    pub(crate) fn unkept(&mut self, root: &str, cx: &App) -> HashSet<TabKey> {
        self.tabs.retain(|(_, watch)| watch(cx).is_some());
        self.tabs
            .iter()
            .filter(|(tab, watch)| tab.repo() == root && watch(cx).is_some_and(|tab| tab.unkept))
            .map(|(tab, _)| tab.clone())
            .collect()
    }

    /// The user confirmed the removal of `root`: the tabs its confirmation
    /// counted now are the ones the removal may close.
    pub(crate) fn confirmed(&mut self, root: &str, cx: &mut Context<Self>) {
        let counted = self.unkept(root, cx);
        self.counted.insert(root.to_owned(), counted);
    }

    /// The tabs the confirmed removal of `root` counted, once it has
    /// landed; none when it counted none.
    pub(crate) fn counted(&mut self, root: &str) -> HashSet<TabKey> {
        self.counted.remove(root).unwrap_or_default()
    }

    /// Whether the tab `tab` holds an edit session of its item, live or
    /// kept, or is getting one ready.
    pub(crate) fn editing(&self, tab: &TabKey, cx: &App) -> bool {
        self.tabs
            .iter()
            .any(|(of, watch)| of == tab && watch(cx).is_some_and(|tab| tab.editing))
    }
}
