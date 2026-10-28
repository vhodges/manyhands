//! Done (design, Done and leaving; decision 15; R-15, R-58, R-73), in the
//! design's order:
//! 1. **During the review** Done is disabled, with its note (`actions`).
//! 2. **A save in flight**: Done waits for its answer. A committed one,
//!    with D2's store call after it, goes on; any other answer stops Done,
//!    and the review or the banner shows it, as Save's states say. A
//!    pending checkpoint already answered, waiting for Retry, stops Done
//!    too. Nothing is discarded.
//! 3. **A never-created item**: Done asks Save, Keep draft, Discard or
//!    Cancel.
//! 4. **Write now**: D2 writes the session's newest revision at once
//!    (`flush_now`), one waiting on its timer included, after any write in
//!    flight; a draft still beginning is waited for (RFC lines 162–163).
//!    The draft counts as written only when D2 holds its newest revision
//!    then: text typed meanwhile is written once more.
//! 5. **Then** (`model::edit::done_outcome`): a clean session ends, its
//!    stored draft discarded; a dirty one whose draft is written is kept
//!    (`kept`); one whose draft is not written asks Save, Discard or Cancel.
//!    Release on leave applies where the session ends
//!    (`release::release_on_leave`).
//!
//! Done's prompts are the leave dialog's (`view::close_draft`). Their Save
//! is the header's, which commits from the text in memory when the draft's
//! write failed (Task 15's ruling); Done then waits for it as in step 2.
//! Their Keep draft closes the tab and leaves the draft for the Drafts
//! page; Discard ends the session, and closes a never-created item's tab,
//! which has no saved copy.

use std::rc::Rc;

use gpui_kit::{App, Context, Window, component::WindowExt};
use manyhands::drafts::DraftId;

use super::{DraftLink, leave::CloseThisTab};
use crate::desktop::{
    model::{
        drafts::RecoveryState,
        edit::{DoneOutcome, Phase, done_outcome},
    },
    view::{
        close_draft::{self, Ask, CloseWithDraft, Offers},
        drafts::actions::may_open,
        item::ItemPane,
    },
};

/// Where Done stands, between its press and its end.
pub(super) enum Finishing {
    /// Waiting for a save's answer: `committed` once it has committed, and
    /// D2 only finishes its store call.
    Save { committed: bool },
    /// Waiting for the draft to begin.
    Begin,
    /// Writing the draft now.
    Write,
    /// Going on, as the next step of the foreground.
    Go,
}

impl ItemPane {
    /// The header's Done, by click or key: not during the review, nor
    /// while Done already goes on.
    pub(super) fn done_requested(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx)
            || !self.editing()
            || self.edit.phase() == Phase::Review
            || self.edit.finishing.is_some()
        {
            return;
        }
        self.done_go(window, cx);
    }

    /// Whether a save is in flight: sent and not answered, settling after
    /// its commit, or D2 still storing the text after it.
    pub(super) fn save_in_flight(&self, cx: &App) -> bool {
        self.edit.phase() == Phase::Saving || self.store_settling(cx)
    }

    /// Done from step 2 on.
    fn done_go(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.edit.finishing = None;
        let Some(session) = self.shown_session() else {
            return;
        };
        if self.edit.phase() == Phase::Review {
            return;
        }
        if self.save_in_flight(cx) {
            let committed = self.edit.phase() != Phase::Saving;
            self.edit.finishing = Some(Finishing::Save { committed });
            cx.notify();
            return;
        }
        // A pending checkpoint, waiting for Retry, still holds the save: it
        // did not commit, so Done stops, and its banner stays in view.
        if self.edit.save.run.is_some() {
            return;
        }
        if session.never_created() {
            self.ask_done(true, window, cx);
            return;
        }
        if session.draft == DraftLink::Beginning {
            self.edit.finishing = Some(Finishing::Begin);
            return;
        }
        let Some(id) = self.live_draft(cx) else {
            self.done_decide(false, window, cx);
            return;
        };
        self.done_write(id, false, window, cx);
    }

    /// Step 4: D2 writes the draft `id` now. Done decides once the newest
    /// revision is stored, as D2's recovery state says then; text typed
    /// while it was written is written once `again`, and otherwise counts
    /// as not written.
    fn done_write(
        &mut self,
        id: DraftId,
        again: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit.finishing = Some(Finishing::Write);
        let flush = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.flush_now(id, cx));
        cx.spawn_in(window, async move |pane, cx| {
            let flushed = flush.await.is_ok();
            let _ = pane.update_in(cx, |pane, window, cx| {
                if !matches!(pane.edit.finishing, Some(Finishing::Write)) {
                    return;
                }
                pane.edit.finishing = None;
                let written = flushed && pane.draft_written(cx);
                match pane.live_draft(cx) {
                    Some(id) if flushed && !written && !again => {
                        pane.done_write(id, true, window, cx)
                    }
                    _ => pane.done_decide(written, window, cx),
                }
            });
        })
        .detach();
    }

    /// Whether the session's draft holds its newest revision: D2 says it
    /// is saved for recovery, with no store call in flight.
    fn draft_written(&self, cx: &App) -> bool {
        let Some(id) = self.live_draft(cx) else {
            return false;
        };
        self.services
            .drafts()
            .read(cx)
            .session(id)
            .is_some_and(|session| {
                !session.busy() && matches!(session.recovery_state(), RecoveryState::Saved(_))
            })
    }

    /// Step 5, once the draft is written as far as it can be: `written`
    /// when its newest revision is stored. A save, or the review, begun
    /// meanwhile stops Done.
    fn done_decide(&mut self, written: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.shown_session() else {
            return;
        };
        if self.edit.phase() != Phase::Editing {
            return;
        }
        let stored = self.live_draft(cx).is_some();
        let outcome = done_outcome(session.never_created(), session.dirty(cx), stored, written);
        match outcome {
            DoneOutcome::End => {
                self.end_session(Some(window), cx);
                self.release_on_leave(cx);
            }
            DoneOutcome::Discard => self.discard_session(Some(window), cx).detach(),
            DoneOutcome::Keep => self.keep_session(window, cx),
            DoneOutcome::AskNotKept => self.ask_done(false, window, cx),
            DoneOutcome::AskNotCreated => self.ask_done(true, window, cx),
        }
    }

    /// The save Done waits on committed: Done goes on once D2 has finished
    /// with it.
    pub(super) fn done_saved(&mut self, cx: &mut Context<Self>) {
        if let Some(Finishing::Save { committed }) = &mut self.edit.finishing {
            *committed = true;
        }
        self.follow_done(cx);
    }

    /// A save, a draft's begin or D2's store call moved: Done, waiting on
    /// it, goes on, or stops on a save that did not commit.
    pub(in crate::desktop::view::item) fn follow_done(&mut self, cx: &mut Context<Self>) {
        let go = match &self.edit.finishing {
            Some(Finishing::Save { committed }) if !self.save_in_flight(cx) => {
                if !*committed {
                    self.edit.finishing = None;
                    cx.notify();
                    return;
                }
                true
            }
            Some(Finishing::Begin) => self
                .edit
                .session
                .as_ref()
                .is_none_or(|session| session.draft != DraftLink::Beginning),
            _ => false,
        };
        if !go {
            return;
        }
        // It goes on with the window, as the next step of the foreground.
        self.edit.finishing = Some(Finishing::Go);
        cx.spawn(async move |pane, cx| {
            let _ = pane.update_in(cx, |pane, window, cx| {
                if matches!(pane.edit.finishing, Some(Finishing::Go)) {
                    pane.done_go(window, cx);
                }
            });
        })
        .detach();
    }

    /// Done asks: a never-created item Save, Keep draft, Discard or Cancel;
    /// otherwise, its draft not written, Save, Discard or Cancel.
    fn ask_done(&mut self, never_created: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !may_open(&self.services, window, cx) {
            return;
        }
        let pane = cx.entity().downgrade();
        let reader = pane.clone();
        let offers = Rc::new(move |cx: &App| {
            reader
                .upgrade()
                .map(|pane| pane.read(cx).done_offers(never_created, cx))
                .unwrap_or(Offers {
                    keep: None,
                    save: Some(false),
                    discard: false,
                })
        });
        close_draft::open(
            Ask::done(never_created, offers),
            window,
            cx,
            move |choice, window, cx| {
                let _ = pane.update(cx, |pane, cx| {
                    pane.done_chosen(choice, never_created, window, cx)
                });
            },
        );
    }

    /// What Done's prompt offers now: Save while the session is dirty and
    /// nothing is saving; Keep draft, for a never-created item, once its
    /// draft is stored.
    fn done_offers(&self, never_created: bool, cx: &App) -> Offers {
        let saving = self.save_in_flight(cx);
        Offers {
            keep: never_created.then(|| self.keep_offer(cx)).flatten(),
            save: Some(self.edit.phase() == Phase::Editing && self.edit_dirty(cx) && !saving),
            discard: !saving,
        }
    }

    /// What Done's prompt chose.
    fn done_chosen(
        &mut self,
        choice: CloseWithDraft,
        never_created: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match choice {
            CloseWithDraft::Cancel => {}
            // Once the dialog has gone, as the header's Save does.
            CloseWithDraft::Save => {
                cx.defer_in(window, |pane, window, cx| pane.done_save(window, cx))
            }
            CloseWithDraft::KeepDraft => self.keep_and_close(cx),
            CloseWithDraft::Discard => {
                let discarded = self.discard_session(Some(window), cx);
                // A never-created item has no saved copy to view.
                cx.spawn(async move |pane, cx| {
                    if discarded.await.is_ok() && never_created {
                        let _ = pane.update(cx, |_, cx| cx.emit(CloseThisTab));
                    }
                })
                .detach();
            }
        }
    }

    /// Save from Done's prompt: the header's Save, and Done waits for it.
    fn done_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.save_requested(window, cx) && self.edit.phase() == Phase::Saving {
            self.edit.finishing = Some(Finishing::Save { committed: false });
        }
    }
}
