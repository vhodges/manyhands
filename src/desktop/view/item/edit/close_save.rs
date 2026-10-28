//! The close dialog's choices, and its Save (design, Done and leaving:
//! Closing the tab; Restored drafts and the close dialog; R-16, R-76).
//!
//! - **What it offers** (`close_offers`), read as each frame is drawn, for
//!   an edit session's dialog and for D2's of a draft restored here. Save
//!   is enabled only when there is something to save (the session is
//!   dirty, or the restored draft holds what the item does not) and
//!   nothing is saving; it is placed after Keep draft, which stays the
//!   default and the first Tab stop. While a save, or Continue editing's
//!   steps before one, run, Keep draft, Save and Discard wait. A new item's
//!   restored draft shows Save unavailable (Task 22).
//! - **Save** saves the session; a restored draft is continued first
//!   (`restored`). The dialog stays open meanwhile. Once the save has
//!   committed and the draft retired, the session ends clean and the tab
//!   closes, with the dialog. Anything else keeps the tab, and the dialog
//!   says what it came to (`close_note`): a pending checkpoint with Retry,
//!   under the same request, a refusal with the library's message and
//!   code, the review, with "Cancel to review the changes." (Task 18's M4:
//!   the review waits behind the dialog), or a prepare or claim that did
//!   not go through.
//! - **Cancel, Keep draft and Discard** end the close's save: a save still
//!   running goes on, but the tab no longer closes when it settles.
//!
//! The note holds fixed words and the library's own message, never draft
//! text.

use std::rc::Rc;

use gpui_kit::{App, Context, Window, component::WindowExt};

use super::{
    CloseThisTab, PREPARE_PENDING, PREPARE_REFUSED, Phase, Stopped, prepare::NOT_PREPARED_AGAIN,
    restored::Then,
};
use crate::desktop::{
    model::edit::save_state::{
        CHANGED_OUTSIDE, CHECK_CURRENT_VERSION, CHECKPOINT_FAILED, SAVE_REFUSED,
        SAVED_INDEX_PENDING, SaveState, save_state,
    },
    model::review::CANCEL_TO_REVIEW,
    view::{
        close_draft::{self, Ask, CloseWithDraft, Offers, Read, SAVING, SaveNote},
        item::{ItemPane, refused_label},
    },
};

impl ItemPane {
    /// Asks before this tab closes with its edit session's unsaved work.
    pub(in crate::desktop::view) fn ask_to_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.entity().downgrade();
        let (offers, note) = self.close_readers(cx);
        close_draft::open(
            Ask::session(offers, note),
            window,
            cx,
            move |choice, _, cx| {
                let _ = pane.update(cx, |pane, cx| pane.close_chosen(choice, cx));
            },
        );
    }

    /// What this tab's close dialog offers and says, read as each frame is
    /// drawn: for its edit session's dialog, or D2's of its restored draft.
    pub(in crate::desktop::view) fn close_readers(
        &self,
        cx: &mut Context<Self>,
    ) -> (Read<Offers>, Read<Option<SaveNote>>) {
        let offering = cx.entity().downgrade();
        let saying = offering.clone();
        let offers = Rc::new(move |cx: &App| {
            offering
                .upgrade()
                .map(|pane| pane.read(cx).close_offers(cx))
                .unwrap_or(Offers {
                    keep: None,
                    save: Some(false),
                    discard: false,
                })
        });
        let note = Rc::new(move |cx: &App| {
            saying
                .upgrade()
                .and_then(|pane| pane.read(cx).close_note(cx))
        });
        (offers, note)
    }

    /// What the close dialog offers now. While a save, or the steps before
    /// one, run, Keep draft and Discard wait, so neither drops the save's
    /// record or races its commit, and Save waits too. D2's dialog's Save
    /// is enabled as the section's is, while Continue editing can start
    /// (Task 18 fix 1, M2).
    pub(in crate::desktop::view) fn close_offers(&self, cx: &App) -> Offers {
        let busy = self.close_busy(cx);
        if self.edit.session.is_none() {
            // D2's dialog of a draft restored here.
            let save = (!self.continues_new_item(cx))
                .then(|| !busy && self.edit_available(cx) && self.restored_unsaved(cx));
            return Offers {
                keep: Some(!busy),
                save,
                discard: !busy,
            };
        }
        Offers {
            keep: self.keep_offer(cx).map(|ready| ready && !busy),
            save: Some(!busy && self.close_save_offered(cx)),
            discard: !busy,
        }
    }

    /// The session's Save in the close dialog: the header's, or, for a
    /// session kept after Done, while it is dirty and nothing saves (Task
    /// 18 fix 1, I2: the design's rule covers a kept draft).
    fn close_save_offered(&self, cx: &App) -> bool {
        if self.session_kept() {
            return self.edit.save.run.is_none() && !self.store_settling(cx) && self.edit_dirty(cx);
        }
        self.save_offered(cx)
    }

    /// Whether a save is in flight, or Continue editing's steps before one
    /// run: a claim, a prepare, or the read before the editor opens. A
    /// prepare waiting for Resume is not running.
    fn close_busy(&self, cx: &App) -> bool {
        let preparing = match self.edit.phase {
            Phase::Preparing(_) => true,
            Phase::Loading => self.edit.stopped.is_none(),
            _ => false,
        };
        self.draft.restoring() || preparing || self.save_in_flight(cx)
    }

    /// What the close dialog's Save came to, while it has not closed the
    /// tab: `None` until it is pressed, and once the dialog has nothing
    /// to say of it.
    pub(in crate::desktop::view) fn close_note(&self, cx: &App) -> Option<SaveNote> {
        // A prepare waiting for Resume is why D2's Save waits (M2): Resume
        // is the tab's banner's, which Cancel returns to.
        if matches!(self.edit.stopped, Some(Stopped::Pending(_))) {
            return Some(SaveNote::says(PREPARE_PENDING));
        }
        if !self.edit.closing {
            return None;
        }
        if self.edit.session.is_none()
            && let Some(refused) = self.draft.refused()
        {
            return Some(SaveNote::says(refused_label(refused)));
        }
        match &self.edit.stopped {
            Some(Stopped::Refused { code, message, .. }) => {
                return Some(SaveNote {
                    words: PREPARE_REFUSED.to_owned(),
                    detail: Some((message.clone(), *code)),
                    retry: false,
                });
            }
            Some(Stopped::Pending(_)) => return Some(SaveNote::says(PREPARE_PENDING)),
            Some(Stopped::NotSentAgain(_)) => return Some(SaveNote::says(NOT_PREPARED_AGAIN)),
            None => {}
        }
        if self.close_busy(cx) {
            return Some(SaveNote::says(SAVING));
        }
        let answer = self.edit.save.answer.as_ref();
        let state = answer.map(|answer| save_state(answer.code, &answer.effects, false));
        // The review the Save opened waits behind the dialog, which stays
        // open with the change, as any other answer keeps it (Task 18's M4):
        // Cancel reaches the review.
        if self.edit.phase == Phase::Review {
            let words = if state == Some(SaveState::Changed) {
                CHANGED_OUTSIDE
            } else {
                CHECK_CURRENT_VERSION
            };
            return Some(SaveNote::says(format!("{words} {CANCEL_TO_REVIEW}")));
        }
        match (state?, answer?) {
            (SaveState::CheckpointFailed, _) => Some(SaveNote {
                words: CHECKPOINT_FAILED.to_owned(),
                detail: None,
                retry: self.edit.save.run.is_some(),
            }),
            (SaveState::Refused(code), answer) => Some(SaveNote {
                words: SAVE_REFUSED.to_owned(),
                detail: Some((answer.message.clone(), code)),
                retry: false,
            }),
            (SaveState::DiscoveryPending, _) => Some(SaveNote::says(SAVED_INDEX_PENDING)),
            _ => None,
        }
    }

    /// What the close dialog chose. Cancel, Keep draft and Discard end the
    /// close's save, and a Save still in Continue editing's steps goes on
    /// as Continue editing alone: a write the user cancelled never happens
    /// (Task 18 fix 1, M1). Save and Retry keep the dialog open.
    pub(in crate::desktop::view) fn close_chosen(
        &mut self,
        choice: CloseWithDraft,
        cx: &mut Context<Self>,
    ) {
        if !matches!(choice, CloseWithDraft::Save | CloseWithDraft::Retry) {
            self.edit.closing = false;
            self.continue_without_saving();
        }
        match choice {
            CloseWithDraft::Cancel => {}
            CloseWithDraft::Save => self.close_save(cx),
            CloseWithDraft::Retry => self.retry_save(cx),
            CloseWithDraft::KeepDraft => self.keep_and_close(cx),
            CloseWithDraft::Discard => {
                let discarded = self.discard_session(None, cx);
                cx.spawn(async move |pane, cx| {
                    if discarded.await.is_ok() {
                        let _ = pane.update(cx, |_, cx| cx.emit(CloseThisTab));
                    }
                })
                .detach();
            }
        }
        cx.notify();
    }

    /// The close dialog's Save: the session saves, a session kept after
    /// Done editing again first, focus left where it is (I2); or a
    /// restored draft is continued, then saved (`restored`).
    fn close_save(&mut self, cx: &mut Context<Self>) {
        self.edit.closing = true;
        if self.edit.session.is_some() {
            self.unkeep(cx);
            self.start_save(cx);
        } else {
            self.continue_requested(Then::Save, cx);
        }
    }

    /// The close's save has settled: committed, the draft retired and the
    /// session clean. The session ends, release on leave applies, and the
    /// tab closes, with its dialog.
    pub(in crate::desktop::view::item) fn follow_close_save(&mut self, cx: &mut Context<Self>) {
        if !self.edit.closing || self.edit.phase != Phase::Editing {
            return;
        }
        let Some(session) = self.shown_session() else {
            return;
        };
        let settled = session.saved()
            && !session.dirty(cx)
            && self.edit.save.run.is_none()
            && !self.store_settling(cx)
            && self.live_draft(cx).is_none();
        if !settled {
            return;
        }
        self.edit.closing = false;
        self.end_session(None, cx);
        self.release_on_leave(cx);
        cx.spawn(async move |pane, cx| {
            let _ = pane.update_in(cx, |_, window, cx| {
                if window.has_active_dialog(cx) {
                    window.close_dialog(cx);
                }
                cx.emit(CloseThisTab);
            });
        })
        .detach();
    }
}
