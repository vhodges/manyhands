//! The item tabs whose edit session holds unsaved text that no draft holds
//! (Part C fix 1, I1; the Cycle's contract 6: a dirty editor is never
//! dropped without the user's choice): crash recovery off, a draft that
//! could not begin, or one still beginning. A repository's removal says how
//! many of its tabs hold such text, and that removing closes them, in
//! whichever view asks for it.
//!
//! Each item tab says so of itself, for as long as it lives (`watch`), as
//! it tells release on leave whether it edits. Nothing here holds text: a
//! tab is named by its repository only.

use gpui_kit::{App, Context};

use crate::desktop::model::stamp::RepoKey;

/// What an item tab says of its edit session: `Some(true)` while it holds
/// unsaved text that no draft holds; `None` once the tab is gone.
pub(crate) type Unkept = Box<dyn Fn(&App) -> Option<bool>>;

/// Every item tab of this window, by its repository.
#[derive(Default)]
pub(crate) struct UnkeptEdits {
    tabs: Vec<(RepoKey, Unkept)>,
}

impl UnkeptEdits {
    /// A tab of `root` says, through `unkept`, whether its edit session
    /// holds text no draft holds.
    pub(crate) fn watch(&mut self, root: RepoKey, unkept: Unkept) {
        self.tabs.push((root, unkept));
    }

    /// How many open tabs of `root` hold unsaved text that no draft holds.
    /// A tab that is gone is forgotten. It is read as a dialog is drawn,
    /// when no tab is being updated.
    pub(crate) fn count(&mut self, root: &str, cx: &mut Context<Self>) -> usize {
        self.tabs.retain(|(_, unkept)| unkept(cx).is_some());
        self.tabs
            .iter()
            .filter(|(of, unkept)| of == root && unkept(cx) == Some(true))
            .count()
    }
}
