//! The save a lifecycle makes first (D5 design, A2, decision 13; R-17):
//! the tab's own Save of its unsaved edits, which writes the draft's newest
//! revision and then sends it (`DraftRegistry::save`: `flush_now`, then the
//! save), its answer shown by the tab as any save's is. A draft of the item
//! stored or restored here with no edit session is unsaved edits too: it is
//! continued and saved, as the close dialog's Save does (`restored`). The
//! lifecycle dialog waits on it (`save_first_state`): a committed save, the
//! draft retired and nothing unsaved, lets it prepare; anything else closes
//! it. Comment drafts are not the item's edits (decision 14).

use gpui_kit::{App, Context};

use super::{Phase, restored::Then};
use crate::desktop::view::item::ItemPane;

/// Where the save before a lifecycle stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SaveFirst {
    /// It runs, or Continue editing's steps before it, or D2 still
    /// finishes it.
    Saving,
    /// It committed, and nothing is unsaved.
    Saved,
    /// It did not commit, or could not start: the tab says why.
    Failed,
}

impl ItemPane {
    /// Saves the tab's unsaved edits before a lifecycle: a session kept
    /// after Done edits again first, and a draft stored or restored here
    /// with no session is continued, then saved, as the close dialog's Save
    /// does. `None` when nothing is unsaved; `Failed` when the save could
    /// not start (an empty required field, a base the review must check
    /// first, or a draft Continue editing cannot take now), the tab saying
    /// why. While the item's close or merge is unfinished, or nothing is
    /// known of it yet, it writes nothing: the lifecycle action stays
    /// available (R-23), but its edits are not committed into a context
    /// the merge has finished with (P23); they stay kept, and the dialog
    /// goes on to prepare. The same holds while nothing is known of the
    /// item yet (`Awaiting`): the edits are then skipped, not saved, and
    /// stay unsaved in the tab.
    pub(in crate::desktop::view) fn start_save_first(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<SaveFirst> {
        if self.edits_wait(cx) {
            return None;
        }
        if self.save_in_flight(cx) {
            return Some(SaveFirst::Saving);
        }
        if self.edit.session.is_none() && self.continued(cx).is_some() {
            if !self.edit_available(cx) {
                return Some(SaveFirst::Failed);
            }
            self.edit.save.first = self.continue_requested(Then::Save, cx);
            return Some(match self.edit.save.first {
                true => SaveFirst::Saving,
                false => SaveFirst::Failed,
            });
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
        if self.close_busy(cx) || self.edit.save.continued {
            return SaveFirst::Saving;
        }
        // Continue editing's editor is open, and its save is about to
        // start: it starts once the frame that opened it is drawn.
        let opening = self.edit.save.first
            && self.edit.stopped.is_none()
            && self.edit.phase() == Phase::Editing
            && self.edit.session.is_some();
        if opening {
            return SaveFirst::Saving;
        }
        // A pending checkpoint keeps its run for Retry; a refusal and an
        // external change leave the edits unsaved, the latter in the
        // review; a refused claim or prepare leaves the draft stored.
        if self.edit.phase() == Phase::Review || self.unsaved(cx) || self.edit.save.run.is_some() {
            SaveFirst::Failed
        } else {
            SaveFirst::Saved
        }
    }

    /// The lifecycle dialog was dismissed while it waited for the save
    /// first: a stored draft still in Continue editing's steps opens in the
    /// editor, but nothing the person has not seen is saved, as the close
    /// dialog's precedent (Task 18 fix 1, M1). A save already sent goes on.
    pub(in crate::desktop::view) fn stop_save_first(&mut self, cx: &mut Context<Self>) {
        self.continue_without_saving();
        self.edit.save.first = false;
        cx.notify();
    }

    /// Whether a press of the lifecycle action could not act now: a save,
    /// a claim, a prepare or Continue editing's steps run, a Discard waits
    /// for its draft, or the review is open (review M-5).
    pub(in crate::desktop::view) fn lifecycle_busy(&self, cx: &App) -> bool {
        self.close_busy(cx)
            || self.edit.save.continued
            || self.edit.discard_begun.is_some()
            || self.edit.phase() == Phase::Review
    }
}
