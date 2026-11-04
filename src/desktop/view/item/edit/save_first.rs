//! The save a lifecycle makes first (D5 design, A2, decision 13): the tab's
//! own Save of its unsaved edits, which writes the draft's newest revision
//! and then sends it (`DraftRegistry::save`), its answer shown by the tab
//! as any save's is. The lifecycle dialog waits on it (`save_first_state`):
//! a committed save, the draft retired and the session clean, lets it
//! prepare; anything else closes it.

use gpui_kit::{App, Context};

use super::Phase;
use crate::desktop::view::item::ItemPane;

/// Where the save before a lifecycle stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SaveFirst {
    /// It runs, or D2 still finishes it.
    Saving,
    /// It committed, and nothing is unsaved.
    Saved,
    /// It did not commit, or could not start: the tab says why.
    Failed,
}

impl ItemPane {
    /// Saves the tab's unsaved edits before a lifecycle: a session kept
    /// after Done edits again first, as the close dialog's Save does. `None`
    /// when the session holds nothing unsaved; `Failed` when the save could
    /// not start (an empty required field, or a base the review must check
    /// first), the tab saying why.
    pub(in crate::desktop::view) fn start_save_first(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<SaveFirst> {
        if self.save_in_flight(cx) {
            return Some(SaveFirst::Saving);
        }
        if !self.edit_dirty(cx) {
            return None;
        }
        self.unkeep(cx);
        Some(match self.start_save(cx) {
            true => SaveFirst::Saving,
            false => SaveFirst::Failed,
        })
    }

    /// Where the save `start_save_first` started stands now.
    pub(in crate::desktop::view) fn save_first_state(&self, cx: &App) -> SaveFirst {
        if self.save_in_flight(cx) {
            return SaveFirst::Saving;
        }
        // A pending checkpoint keeps its run for Retry; a refusal and an
        // external change leave the session dirty, the latter in the
        // review.
        if self.edit.phase() == Phase::Review || self.edit_dirty(cx) || self.edit.save.run.is_some()
        {
            SaveFirst::Failed
        } else {
            SaveFirst::Saved
        }
    }
}
