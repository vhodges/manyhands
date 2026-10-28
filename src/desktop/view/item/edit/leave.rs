//! Leaving an edit session (design, Done and leaving; R-15, R-41, R-71):
//! the session ended or discarded, closing the tab, and a session a
//! discard elsewhere ended.
//!
//! - **Closing the tab.** Unsaved work, a kept or stored draft, or a
//!   draft still being stored asks first (R-41), in the leave dialog
//!   (`view::close_draft`): Keep draft where a draft is stored, waiting
//!   while one is being stored; Discard and Cancel always; Keep draft and
//!   Discard waiting while a save is in flight; Save shown unavailable
//!   until the close dialog saves (Task 18). The message follows what is
//!   offered. A clean tab with no draft closes at once.
//!   Keep draft closes the tab once the draft is written and let go of; a
//!   failed write keeps the tab, with the failure in the toolbar. Discard
//!   closes it once the draft is deleted; a failed discard keeps it open
//!   and says so, the draft let go of and listed again.
//! - **Ended elsewhere** (R-71). When D2 discards the session's draft for
//!   a reason the tab did not cause, from the Drafts page, a kept session
//!   is dropped and a live one ends: the tab is viewing the saved copy and,
//!   once the discard lands, says "The draft was discarded.". A discard the
//!   store refused says nothing: D2's banner offers the draft, still
//!   stored. A never-created item, which has no saved copy, closes its tab
//!   once the discard lands. A draft of the item still stored is
//!   offered by D2's draft banner.
//! - **Release on leave** (`release`): a tab that leaves clean hands its
//!   item over; a discard of a stored draft reaches it through the
//!   registry, once the store has deleted the draft.
//!
//! The tab's own discards let go of the session before D2 discards, so
//! the registry's `SessionDiscarded` finds none to end.

use std::rc::Rc;

use futures::channel::oneshot;
use gpui_kit::{App, Context, EventEmitter, Task, Window};
use manyhands::drafts::{DraftErrorKind, DraftId};

use super::DraftLink;
use crate::desktop::{
    model::edit::Phase,
    services::drafts::SessionDiscarded,
    view::{
        close_draft::{self, Ask, CloseWithDraft, Offers},
        item::ItemPane,
    },
};

/// What closing an item tab asks (design, Done and leaving: Closing the
/// tab).
pub(crate) enum Leaving {
    /// Nothing is unsaved: the tab closes at once.
    Clean,
    /// A draft restored in the tab, with no edit session: D2's dialog.
    Restored(DraftId),
    /// An edit session with unsaved work, a kept or stored draft, or a
    /// draft being stored (R-41): the tab asks (`ask_to_close`).
    Session,
}

/// The tab is to close without asking again: its choice was made, or a
/// discard elsewhere ended a never-created item's session (R-71).
pub(crate) struct CloseThisTab;

/// A discard elsewhere that ended the tab's session (R-71).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Elsewhere {
    /// The store call lands next; a never-created item's tab closes then.
    Landing { never_created: bool },
    /// The draft is gone: the banner says so until the next Edit.
    Discarded,
}

impl EventEmitter<CloseThisTab> for ItemPane {}

impl ItemPane {
    /// What closing this tab asks.
    pub(in crate::desktop::view) fn leaving(&self, cx: &App) -> Leaving {
        let attached = self.services.drafts().read(cx).attached(&self.tab_key());
        match &self.edit.session {
            Some(session)
                if session.dirty(cx)
                    || session.kept
                    || attached.is_some()
                    || session.draft == DraftLink::Beginning =>
            {
                Leaving::Session
            }
            Some(_) => Leaving::Clean,
            None => attached.map_or(Leaving::Clean, Leaving::Restored),
        }
    }

    /// Whether the edit session is dirty: the tab's unsaved mark shows it
    /// whether or not a draft is stored (design, Done and leaving).
    pub(in crate::desktop::view) fn unsaved_edit(&self, cx: &App) -> bool {
        self.edit_dirty(cx)
    }

    /// Asks before this tab closes with its edit session's unsaved work.
    pub(in crate::desktop::view) fn ask_to_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.entity().downgrade();
        let reader = pane.clone();
        let offers = Rc::new(move |cx: &App| {
            reader
                .upgrade()
                .map(|pane| pane.read(cx).close_offers(cx))
                .unwrap_or(Offers {
                    keep: None,
                    save: None,
                    discard: false,
                })
        });
        close_draft::open(Ask::session(offers), window, cx, move |choice, _, cx| {
            let _ = pane.update(cx, |pane, cx| pane.close_chosen(choice, cx));
        });
    }

    /// What the close dialog offers now. While a save is in flight, Keep
    /// draft and Discard wait, so neither drops the save's record or races
    /// its commit.
    fn close_offers(&self, cx: &App) -> Offers {
        let saving = self.save_in_flight(cx);
        Offers {
            keep: self.keep_offer(cx).map(|ready| ready && !saving),
            save: None,
            discard: !saving,
        }
    }

    /// Keep draft: offered while a draft of the session is stored, and
    /// waiting while its first write is in flight.
    pub(super) fn keep_offer(&self, cx: &App) -> Option<bool> {
        let session = self.edit.session.as_ref()?;
        if self.live_draft(cx).is_some() {
            Some(true)
        } else {
            (session.draft == DraftLink::Beginning).then_some(false)
        }
    }

    /// What the close dialog chose.
    fn close_chosen(&mut self, choice: CloseWithDraft, cx: &mut Context<Self>) {
        match choice {
            CloseWithDraft::Cancel | CloseWithDraft::Save => {}
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
    }

    /// Keep draft: the newest text is written and the draft let go of,
    /// stored for the Drafts page; then the tab closes. A failed write
    /// keeps the session, with the failure in the toolbar.
    pub(super) fn keep_and_close(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.live_draft(cx) else {
            return;
        };
        let release = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.release(id, cx));
        cx.spawn(async move |pane, cx| {
            if release.await.is_ok() {
                let _ = pane.update(cx, |pane, cx| {
                    pane.end_session(None, cx);
                    cx.emit(CloseThisTab);
                });
            }
        })
        .detach();
    }

    /// The session ends: the tab is viewing the saved copy, and, given a
    /// window, the page's read editor takes the focus. Nothing of a save of
    /// it is followed any more.
    pub(super) fn end_session(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        if self.edit.session.take().is_none() {
            return;
        }
        self.edit.phase = Phase::Viewing;
        self.edit.finishing = None;
        self.edit.kept_changed = None;
        self.edit.save.clear();
        if let Some(window) = window {
            self.page.focus_read(window, cx);
        }
        cx.notify();
    }

    /// Ends the session and discards its draft, if one is stored. Release
    /// on leave applies at once when none is, and otherwise once the
    /// discard has landed and deleted it, as the registry reports
    /// (`DraftDiscardLanded`). A failed discard has released the draft,
    /// still stored: the banner says so.
    pub(super) fn discard_session(
        &mut self,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        let draft = self.live_draft(cx);
        let beginning = self
            .edit
            .session
            .as_ref()
            .is_some_and(|session| session.draft == DraftLink::Beginning);
        self.end_session(window, cx);
        // M4: a draft still beginning is waited for, then discarded, and
        // only then is release on leave handed anything (`begun_discarded`).
        if beginning {
            let (sender, receiver) = oneshot::channel();
            self.edit.discard_begun = Some(sender);
            return cx.spawn(async move |_, _| receiver.await.unwrap_or(Err(DraftErrorKind::Io)));
        }
        let Some(id) = draft else {
            self.release_on_leave(cx);
            return Task::ready(Ok(()));
        };
        let discard = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.discard(id, cx));
        cx.spawn(async move |pane, cx| {
            let discarded = discard.await;
            if let Err(kind) = discarded {
                let _ = pane.update(cx, |pane, cx| pane.discard_failed(kind, cx));
            }
            discarded
        })
    }

    /// M4: the begin a Discard waited for landed, the session already
    /// ended. A draft it stored is discarded, and the registry reports the
    /// discard to release on leave; with none stored, the tab leaves clean
    /// now. False when no Discard waited.
    pub(super) fn begun_discarded(
        &mut self,
        begun: Result<DraftId, DraftErrorKind>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(waiting) = self.edit.discard_begun.take() else {
            return false;
        };
        let Ok(id) = begun else {
            self.release_on_leave(cx);
            let _ = waiting.send(Ok(()));
            return true;
        };
        let discard = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.discard(id, cx));
        cx.spawn(async move |pane, cx| {
            let discarded = discard.await;
            if let Err(kind) = discarded {
                let _ = pane.update(cx, |pane, cx| pane.discard_failed(kind, cx));
            }
            let _ = waiting.send(discarded);
        })
        .detach();
        true
    }

    /// R-71: D2 discarded the draft of the session attached to `event`'s
    /// tab. When it is this tab's, and the tab still holds its session (it
    /// did not cause the discard), the session ends at once. Once the
    /// discard lands, the tab says the draft was discarded, and release on
    /// leave applies, as the registry reports the discard to it
    /// (`DraftDiscardLanded`); a discard the store refused leaves the draft
    /// stored, for D2's banner to offer, and the tab says nothing of it. A
    /// never-created item, with no saved copy to view, closes its tab then.
    pub(in crate::desktop::view::item) fn session_discarded(
        &mut self,
        event: &SessionDiscarded,
        cx: &mut Context<Self>,
    ) {
        match event {
            SessionDiscarded::Ended(tab) if *tab == self.tab_key() => {
                let Some(session) = &self.edit.session else {
                    return;
                };
                let never_created = session.never_created();
                self.end_session(None, cx);
                self.edit.discarded = Some(Elsewhere::Landing { never_created });
            }
            SessionDiscarded::Landed(tab, discarded) if *tab == self.tab_key() => {
                let Some(Elsewhere::Landing { never_created }) = self.edit.discarded else {
                    return;
                };
                self.edit.discarded = discarded.then_some(Elsewhere::Discarded);
                if never_created {
                    cx.emit(CloseThisTab);
                }
                cx.notify();
            }
            SessionDiscarded::Ended(_) | SessionDiscarded::Landed(..) => {}
        }
    }
}
