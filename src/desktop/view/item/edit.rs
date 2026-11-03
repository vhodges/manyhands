//! Edit (design, Part C, Phases, steps 1 to 6, Done and leaving, and
//! Restored drafts and the close dialog): the item's private work context
//! made ready, the edit editor opened on a fresh read, D2's draft begun and
//! fed as the text changes, the text saved, and the session left.
//!
//! - **Viewing.** Edit, `e` on the page and `secondary-shift-e` in the tab
//!   start it, where it is offered (`model::edit::edit_offered`). While a
//!   draft of the item is kept or stored it reads Continue editing (R-35):
//!   a kept session shows its editor again, and a stored or restored draft
//!   takes the restored-draft route (`restored`, R-42). A new item's draft
//!   whose item is not created yet is continued with the Create intent and
//!   no item read, and its Save is its create (`create`).
//! - **Preparing.** `context prepare` with the Edit intent runs on the user
//!   lane (`prepare`). The pill says the private copy is being made ready,
//!   Edit is disabled, and the page stays readable. A tab closed meanwhile
//!   opens nothing; a context the answer made is still counted,
//!   process-wide (`services::created_contexts`).
//! - **Loading.** On `ok` the item is read again under a new stamp (R-70),
//!   and the edit editor opens from that read (`open`): on the body as the
//!   editor loads it, with the read editor's caret when the read editor
//!   holds that text (R-65), or on a continued draft's text, exactly
//!   (R-79); the toolbar above the page and focus in the editor. A prepare
//!   that answered `discovery_pending` waits in Loading for Resume, which
//!   sends the same request again (R-13, R-77): the pill still says the
//!   copy is being made ready, and no read opens the editor meanwhile. A
//!   refusal is shown in a banner, and the tab stays viewing.
//! - **Editing.** The first difference from the base, of the body or of
//!   any other field, begins D2's draft, on the base the read gave (R-11);
//!   text typed before it lands is written with it, and every later
//!   difference, undo's included (R-12), updates it. The draft is attached
//!   to the tab. A draft store that did not open is said before editing
//!   starts, and the user chooses whether to go on (`recovery_off`). The
//!   toolbar's status item says what D2 holds of the text. A late read
//!   never replaces a dirty or drafted editor (`late`, contract 6).
//! - **Saving** and the session after a save are `save` and `saved`, with
//!   their button, pill and banners in `save_view`.
//! - **Done** (`done`) waits for a save in flight, writes the draft now,
//!   and ends the session, keeps it (`kept`: the tab views the saved copy
//!   while the edit editor and its undo stay, R-58), or asks. A session
//!   ended elsewhere, and closing the tab, are `leave`; the close dialog's
//!   Save is `close_save`.
//! - **Review** (`review`, drawn by `review_view`): the stale-edit review
//!   takes the page's place, with the draft feed suspended, until Use
//!   these choices or Cancel (decision 4).
//!
//! Release on leave (`release`) hands the tab's item to the services' release
//! when the tab leaves clean, and offers the release of a stranded context
//! (R-40). Draft text stays in the editor, the registry and the store;
//! nothing here derives `Debug`.

mod actions;
mod close_save;
mod create;
mod done;
mod feed;
mod kept;
mod late;
mod leave;
mod open;
mod prepare;
mod recovery_off;
mod release;
mod restored;
mod review;
mod review_view;
mod save;
mod save_view;
mod saved;

pub(crate) use self::leave::{CloseThisTab, Leaving};
#[cfg(all(test, feature = "desktop-test-support"))]
pub(super) use self::recovery_off::{CONTINUE, message as recovery_off_message};
use self::restored::{Continuing, Then};
pub(crate) use self::save_view::InitialsInSettings;

use futures::{channel::oneshot, future::Shared};
use gpui_kit::{
    AnyElement, App, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Styled, Subscription, TestSupportExt, Window, component::WindowExt, div,
};
use manyhands::{
    drafts::{DraftBase, DraftErrorKind, DraftFields, DraftId, is_new_item_base},
    results::ResultCode,
};
use zorite_editor::EditorState;

use super::ItemPane;
use crate::desktop::{
    model::{
        edit::{Phase, edit_offered, fields_with_body},
        stamp::ViewScope,
        tabs::TabKey,
    },
    services::requests::DesktopRequestId,
    view::{
        close_draft::Opened,
        editor::{host::EditorHost, toolbar::Toolbar},
    },
};

/// A prepare that was refused: the step, then the library's message and
/// code (design, Banners: another refusal).
pub(crate) const PREPARE_REFUSED: &str =
    "Could not get a private copy ready. Your saved work is unchanged.";
/// A prepare whose hand-off to the index is still owed (R-13).
pub(crate) const PREPARE_PENDING: &str =
    "Your private copy is made, but the search index is not updated yet. Nothing is lost.";
pub(crate) use crate::desktop::model::edit::save_state::{RESUME, RETRY};

/// Where an Edit that did not reach the editor stopped.
enum Stopped {
    /// `discovery_pending`: Resume sends this request again (R-77).
    Pending(DesktopRequestId),
    /// Refused, in the library's words; `retry` when its recovery offers
    /// the same request again.
    Refused {
        code: ResultCode,
        message: String,
        retry: Option<DesktopRequestId>,
    },
    /// Resume or Retry found the request not to be sent again, a release
    /// of the item having been sent since (R-78): the tab is viewing.
    NotSentAgain(DesktopRequestId),
}

/// The session's D2 draft.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DraftLink {
    /// Nothing differs from the base yet.
    None,
    /// `begin` runs; edits meanwhile are written once it lands.
    Beginning,
    Live(DraftId),
    /// `begin` failed: the text is the editor's only, and the next
    /// difference begins again.
    Failed,
}

/// An edit session: the edit editor, its toolbar, the base its draft
/// begins on, the fields besides the body, and the draft. It holds the
/// user's text, so it derives nothing.
pub(super) struct EditSession {
    host: Entity<EditorHost>,
    toolbar: Entity<Toolbar>,
    /// The item as read when the editor opened, normalized (R-11).
    base: DraftBase,
    /// The session's fields besides the body, which the editor holds: the
    /// base's, until the details sheet's controls change one (Task 20).
    meta: DraftFields,
    draft: DraftLink,
    /// A save has committed in this session.
    saved: bool,
    /// Whether the session was dirty when last fed: the tab redraws when it
    /// changes.
    dirty_seen: bool,
    /// The session is kept after Done: the tab views the saved copy, and
    /// the edit editor waits, with its text, caret and undo (R-58).
    kept: bool,
    _subscriptions: [Subscription; 2],
}

impl EditSession {
    /// Whether a save has committed in this session.
    pub(super) fn saved(&self) -> bool {
        self.saved
    }

    /// Whether the session is kept after Done.
    pub(super) fn kept(&self) -> bool {
        self.kept
    }

    /// The base the session's next draft begins on.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(super) fn base(&self) -> &DraftBase {
        &self.base
    }

    /// The session's fields now: the editor's text over the fields besides
    /// it.
    fn current(&self, cx: &App) -> DraftFields {
        fields_with_body(&self.meta, self.host.read(cx).text(cx))
    }

    /// Whether the item was never created: its base is a new item's (R-20),
    /// and it is dirty until its create succeeds (R-73).
    fn never_created(&self) -> bool {
        is_new_item_base(&self.base)
    }

    /// Dirty (design, Done and leaving): the text or any other field differs
    /// from the baseline, the saved copy the session's base holds (R-69),
    /// or the item is not yet created. It is the difference that begins a
    /// draft (`feed`). The editor's own flag is not it: its baseline moves
    /// only when a save leaves the editor holding what was sent.
    fn dirty(&self, cx: &App) -> bool {
        self.current(cx) != self.base.fields || self.never_created()
    }
}

/// A tab's edit state.
pub(super) struct TabEdit {
    phase: Phase,
    /// A read landed while Loading: the editor opens from it as the next
    /// frame is drawn.
    loaded: bool,
    stopped: Option<Stopped>,
    session: Option<EditSession>,
    /// The header's Done, where Escape in the editor goes (R-59).
    done: FocusHandle,
    /// Where Done stands, once pressed and until it ends, keeps or asks.
    finishing: Option<done::Finishing>,
    /// A discard elsewhere that ended the tab's session (R-71), while it
    /// lands and once the draft is gone.
    discarded: Option<leave::Elsewhere>,
    /// The kept-draft banner's Discard…, where its dialog gives the focus
    /// back.
    kept_discard: FocusHandle,
    /// Whether the session's item changed under its draft, kept or shown,
    /// as last classified (R-68): for the draft and revision it was, until
    /// a read lands.
    changed: Option<(DraftId, u64, bool)>,
    /// A Discard waiting for the draft still beginning (`leave`).
    discard_begun: Option<(u64, oneshot::Sender<Result<(), DraftErrorKind>>)>,
    /// How many begins the tab has sent: the token a waiting Discard is
    /// answered by.
    begins: u64,
    /// The latest begin, which resolves once it has landed: quitting waits
    /// for a begin still in flight (Part C fix 1, I1).
    begin_landing: Option<Shared<oneshot::Receiver<()>>>,
    /// The attempt of the prepare whose answer was taken, or that Resume
    /// or Retry sent again over: only a later attempt's answer is taken
    /// (Part C fix 1, M5).
    prepare_taken: u32,
    save: save::TabSave,
    /// Continue editing's draft, from the prepare until the editor opens
    /// on it (`restored`).
    continuing: Option<Continuing>,
    /// The close dialog's Save runs: the tab closes once the draft retires
    /// (`close_save`).
    closing: bool,
    /// The tab's close dialog, last opened (Task 27 fix 1, A2).
    close_dialog: Option<Opened>,
    /// The stale-edit review, while it is open (`review`).
    review: Option<review::ReviewState>,
    /// What stands in for a new item in this tab until its read first
    /// finds it (`create`).
    stand_in: create::StandIn,
}

impl TabEdit {
    /// The edit state of a tab whose reads are stamped under `scope`.
    pub(super) fn new(scope: ViewScope, cx: &mut App) -> Self {
        Self {
            phase: Phase::Viewing,
            loaded: false,
            stopped: None,
            session: None,
            done: cx.focus_handle().tab_stop(true),
            finishing: None,
            discarded: None,
            kept_discard: cx.focus_handle(),
            changed: None,
            discard_begun: None,
            begins: 0,
            begin_landing: None,
            prepare_taken: 0,
            save: save::TabSave::new(scope),
            continuing: None,
            closing: false,
            close_dialog: None,
            review: None,
            stand_in: create::StandIn::default(),
        }
    }

    pub(super) fn phase(&self) -> Phase {
        self.phase
    }

    /// A read landed, `found` when it found the item: while Loading, and
    /// not waiting for Resume, the editor opens from it; a review still
    /// waiting for its read takes it, the one it opened on or a newer one
    /// that superseded it (I1); a session's draft is classified against it
    /// again; and once the item is found, nothing stands in for it again.
    pub(super) fn read_landed(&mut self, found: bool) {
        self.stand_in.read_landed(found);
        if self.phase == Phase::Loading && self.stopped.is_none() {
            self.loaded = true;
        }
        if let Some(review) = &mut self.review
            && review.model.is_none()
        {
            review.landed = true;
        }
        self.changed = None;
    }
}

impl ItemPane {
    /// The edit session, while one lives, kept or not.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(super) fn edit_session(&self) -> Option<&EditSession> {
        self.edit.session.as_ref()
    }

    /// The edit editor's host, while a session lives, kept or not.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn edit_host(&self) -> Option<Entity<EditorHost>> {
        Some(self.edit.session.as_ref()?.host.clone())
    }

    /// Where the tab stands.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn edit_phase(&self) -> Phase {
        self.edit.phase
    }

    /// The review's body editor, once the body has a result.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn review_host(&self) -> Option<Entity<EditorHost>> {
        self.edit.review.as_ref()?.host.clone()
    }

    /// The edit editor's toolbar, while a session lives.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn edit_toolbar(&self) -> Option<Entity<Toolbar>> {
        Some(self.edit.session.as_ref()?.toolbar.clone())
    }

    /// The edit editor, while it is shown: not while the review takes its
    /// place.
    pub(super) fn edit_editor(&self, cx: &App) -> Option<Entity<EditorState>> {
        if self.edit.phase == Phase::Review {
            return None;
        }
        let session = self.shown_session()?;
        Some(session.host.read(cx).editor().clone())
    }

    /// Whether the edit editor is shown: Editing, Saving or the Review, not
    /// while the session is kept.
    pub(super) fn editing(&self) -> bool {
        self.shown_session().is_some()
    }

    /// Whether an edit session lives, kept or not: its draft is the edit
    /// editor's, never D2's restored section.
    pub(in crate::desktop::view) fn has_session(&self) -> bool {
        self.edit.session.is_some()
    }

    /// The shown session's fields besides the body, which the details
    /// sheet's controls change: not a kept session's, whose tab views the
    /// saved copy.
    pub(super) fn session_meta(&self) -> Option<&DraftFields> {
        Some(&self.shown_session()?.meta)
    }

    /// The session's fields besides the body and its base's, kept or not:
    /// what a save of it would send, and against what.
    pub(super) fn session_fields(&self) -> Option<(&DraftFields, &DraftFields)> {
        let session = self.edit.session.as_ref()?;
        Some((&session.meta, &session.base.fields))
    }

    /// Whether the session is dirty: its text or another field differs from
    /// the baseline, or its item is not created yet.
    pub(super) fn edit_dirty(&self, cx: &App) -> bool {
        self.edit
            .session
            .as_ref()
            .is_some_and(|session| session.dirty(cx))
    }

    /// Edit, Continue editing, `e` and `secondary-shift-e`. False when
    /// nothing was started, so a key goes on.
    pub(super) fn edit_requested(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // A dialog in front of the tab takes every key (controller ruling,
        // 2026-10-27).
        if window.has_active_dialog(cx) || !self.edit_available(cx) {
            return false;
        }
        // R-58: a kept session shows its editor again; nothing is prepared.
        if self.resume_kept(window, cx) {
            return true;
        }
        if !matches!(self.tab_key(), TabKey::Item { .. }) {
            return false;
        }
        self.edit.discarded = None;
        // R-35: a draft stored or restored here is continued, not a second
        // one begun (`restored`).
        if self.continue_requested(Then::Edit, cx) {
            return true;
        }
        self.edit.continuing = None;
        match self.services.drafts().read(cx).unavailable() {
            Some(reason) => {
                let pane = cx.entity().downgrade();
                recovery_off::open(reason, window, cx, move |cx| {
                    let _ = pane.update(cx, |pane, cx| pane.prepare(cx));
                });
            }
            None => self.prepare(cx),
        }
        true
    }

    /// Whether Edit can start now: viewing an item it is offered for; a
    /// kept session is continued, and so is a draft stored or restored here.
    /// A new item's draft whose item is not created yet is continued with
    /// no item to read (R-42, `create`). A prepare waiting for Resume is in
    /// Loading. Not while a Discard waits for its draft (re-review N2), nor
    /// while a draft is being claimed.
    pub(super) fn edit_available(&self, cx: &App) -> bool {
        let viewing = self.edit.phase == Phase::Viewing
            && self.edit.discard_begun.is_none()
            && !self.draft.restoring();
        viewing
            && match self.model.item().rows() {
                Some(item) => edit_offered(self.model.target(), item),
                None => self.continued_not_created(cx),
            }
    }

    /// The edit session while its editor is shown, not kept.
    fn shown_session(&self) -> Option<&EditSession> {
        self.edit.session.as_ref().filter(|session| !session.kept())
    }

    /// Whether a session is kept after Done.
    fn session_kept(&self) -> bool {
        self.edit.session.as_ref().is_some_and(EditSession::kept)
    }

    /// The mode key, from anywhere in the tab while editing: the host
    /// switches, the review's body editor while the review is open. False
    /// when no editor is shown, so the key goes on.
    pub(super) fn toggle_rich(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = self.shown_session() else {
            return false;
        };
        let host = match &self.edit.review {
            Some(review) => review.host.clone(),
            None => Some(session.host.clone()),
        };
        if let Some(host) = host {
            host.update(cx, |host, cx| host.set_rich(!host.is_rich(), cx));
        }
        true
    }

    /// The toolbar, under the header while editing; hidden while the
    /// review is open.
    pub(super) fn toolbar_view(&self) -> Option<AnyElement> {
        if self.edit.phase == Phase::Review {
            return None;
        }
        let session = self.shown_session()?;
        Some(
            div()
                .id("item-toolbar")
                .test_support()
                .flex_none()
                .w_full()
                .child(session.toolbar.clone())
                .into_any_element(),
        )
    }

    /// The page's edit editor, while editing, or the review in its place
    /// while that is open; a kept session's page is the saved copy's.
    pub(super) fn edit_view(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let session = self.shown_session()?;
        if self.edit.phase == Phase::Review
            && let Some(review) = &self.edit.review
        {
            return Some(self.review_view(review, cx));
        }
        Some(self.page.edit_view(&session.host))
    }
}
