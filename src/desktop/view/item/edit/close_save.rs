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
//!   restored draft is offered Save too: it is created (R-42, `create`).
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
    model::drafts::RecoveryState,
    model::edit::save_state::{
        CHANGED_OUTSIDE, CHECK_CURRENT_VERSION, CHECKPOINT_FAILED, SAVE_REFUSED,
        SAVED_INDEX_PENDING, SaveState, save_state,
    },
    model::review::CANCEL_TO_REVIEW,
    view::{
        close_draft::{self, Ask, CloseWithDraft, Offers, Opened, Read, SAVING, SaveNote},
        item::{ItemPane, refused_label},
    },
};

impl ItemPane {
    /// Asks before this tab closes with its edit session's unsaved work.
    /// A clean session whose draft is stored, or being stored, asks too,
    /// under D2's title for a tab with a draft, in words that say it has no
    /// unsaved changes and name nothing (Part C fix 1, M2; fix 2, N3). While
    /// the draft's last write failed, the words say Keep draft tries again.
    pub(in crate::desktop::view) fn ask_to_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.entity().downgrade();
        let (offers, note) = self.close_readers(cx);
        let ask = if self.edit_dirty(cx) || self.session_kept() {
            let reading = pane.clone();
            let failed = Rc::new(move |cx: &App| {
                reading
                    .upgrade()
                    .is_some_and(|pane| pane.read(cx).last_write_failed(cx))
            });
            Ask::session(offers, note, failed)
        } else {
            Ask::session_draft(offers, note)
        };
        let opened = close_draft::open(ask, window, cx, move |choice, _, cx| {
            let _ = pane.update(cx, |pane, cx| pane.close_chosen(choice, cx));
        });
        self.close_dialog_opened(opened);
    }

    /// The tab's close dialog opened, here or by the workspace for a draft
    /// restored here: its Save closes it once settled (`follow_close_save`).
    pub(in crate::desktop::view) fn close_dialog_opened(&mut self, opened: Opened) {
        self.edit.close_dialog = Some(opened);
    }

    /// Whether the last write of the session's draft failed: its newest
    /// text is the editor's only.
    fn last_write_failed(&self, cx: &App) -> bool {
        self.live_draft(cx)
            .and_then(|id| self.services.drafts().read(cx).recovery_state(id))
            .is_some_and(|state| matches!(state, RecoveryState::Failed(_)))
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
            // D2's dialog of a draft restored here; a new item's is created
            // (R-42).
            let save = !busy
                && self.edit_available(cx)
                && self.restored_unsaved(cx)
                && self.save_blocked(cx).is_none();
            return Offers {
                keep: Some(!busy),
                save: Some(save),
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
            return self.save_ready(cx);
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

    /// What the close dialog says of its Save, while it has not closed the
    /// tab. While the review is open, how to reach it; once Save is
    /// pressed, what it came to (`closing_note`); else, only when nothing
    /// else blocks it, the empty required field that does (Task 20 fix 1).
    /// `None` when there is nothing to say.
    pub(in crate::desktop::view) fn close_note(&self, cx: &App) -> Option<SaveNote> {
        // A prepare waiting for Resume is why D2's Save waits (M2): Resume
        // is the tab's banner's, which Cancel returns to.
        if matches!(self.edit.stopped, Some(Stopped::Pending(_))) {
            return Some(SaveNote::says(PREPARE_PENDING));
        }
        // The review waits behind the dialog, whether the dialog's own Save
        // opened it (Task 18's M4) or it was open already (Task 19 fix 1,
        // M2): Cancel reaches it, and Keep draft or Discard leaves it.
        if self.edit.phase == Phase::Review {
            let answered = self.edit.save.answer.as_ref().map(|answer| {
                save_state(answer.code, &answer.effects, false) == SaveState::Changed
            });
            let changed = answered == Some(true) || matches!(self.edit.changed, Some((_, _, true)));
            let words = if changed {
                CHANGED_OUTSIDE
            } else {
                CHECK_CURRENT_VERSION
            };
            return Some(SaveNote::says(format!("{words} {CANCEL_TO_REVIEW}")));
        }
        if self.edit.closing
            && let Some(note) = self.closing_note(cx)
        {
            return Some(note);
        }
        self.save_blocked(cx).map(SaveNote::says)
    }

    /// What the close dialog's Save came to, once pressed: a refused claim
    /// or prepare, a prepare waiting or not sent again, the save running,
    /// or its answer. `None` once there is nothing to say of it.
    fn closing_note(&self, cx: &App) -> Option<SaveNote> {
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
    /// tab closes, as was chosen. Its dialog closes itself, and only while
    /// focus is in it, as Task 23 ruled for Create, Move and New folder: the
    /// Kit closes only the topmost dialog, and a credential prompt may have
    /// opened above it. Left open over a closed tab, it offers Cancel alone
    /// until Escape or Cancel. A dialog that went without a choice, as
    /// quitting closes every dialog, ends the close: its save, settling,
    /// closes nothing (Task 27 fix 1, A2).
    pub(in crate::desktop::view::item) fn follow_close_save(&mut self, cx: &mut Context<Self>) {
        if self.edit.close_dialog.as_ref().is_none_or(Opened::gone) {
            self.edit.closing = false;
        }
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
        let dialog = self.edit.close_dialog.take();
        self.end_session(None, cx);
        self.release_on_leave(cx);
        cx.spawn(async move |pane, cx| {
            let _ = pane.update_in(cx, |_, window, cx| {
                if dialog.is_some_and(|dialog| dialog.focused(window, cx)) {
                    window.close_dialog(cx);
                }
                cx.emit(CloseThisTab);
            });
        })
        .detach();
    }
}
