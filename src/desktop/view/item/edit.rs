//! Edit (design, Part C, Phases, steps 1 to 6, and Done and leaving): the
//! item's private work context made ready, the edit editor opened on a
//! fresh read, D2's draft begun and fed as the text changes, the text
//! saved, and the session left.
//!
//! - **Viewing.** Edit, `e` on the page and `secondary-shift-e` in the tab
//!   start it, where it is offered (`model::edit::edit_offered`). While a
//!   draft of the item is kept or stored it reads Continue editing (R-35).
//! - **Preparing.** `context prepare` with the Edit intent runs on the user
//!   lane (`prepare`). The pill says the private copy is being made ready,
//!   Edit is disabled, and the page stays readable. A tab closed meanwhile
//!   opens nothing; a context the answer made is still counted,
//!   process-wide (`services::created_contexts`).
//! - **Loading.** On `ok` the item is read again under a new stamp (R-70),
//!   and the edit editor opens from that read: on the body as the editor
//!   loads it, with the read editor's caret when the read editor holds that
//!   text (R-65), the toolbar above the page and focus in the editor. A
//!   prepare that answered `discovery_pending` waits in Loading for Resume,
//!   which sends the same request again (R-13, R-77): the pill still says
//!   the copy is being made ready, and no read opens the editor meanwhile.
//!   A refusal is shown in a banner, and the tab stays viewing.
//! - **Editing.** The first difference from the base, of the body or of
//!   any other field, begins D2's draft, on the base the read gave (R-11);
//!   text typed before it lands is written with it, and every later
//!   difference, undo's included (R-12), updates it. The draft is attached
//!   to the tab. A draft store that did not open is said before editing
//!   starts, and the user chooses whether to go on (`recovery_off`). The
//!   toolbar's status item says what D2 holds of the text.
//! - **Saving** and the session after a save are `save` and `saved`, with
//!   their button, pill and banners in `save_view`.
//! - **Done** (`done`) waits for a save in flight, writes the draft now,
//!   and ends the session, keeps it (`kept`: the tab views the saved copy
//!   while the edit editor and its undo stay, R-58), or asks. A session
//!   ended elsewhere, and closing the tab, are `leave`.
//!
//! Release on leave (`release`) hands the tab's item to the services' release
//! when the tab leaves clean, and offers the release of a stranded context
//! (R-40). Draft text stays in the editor, the registry and the store;
//! nothing here derives `Debug`.

mod actions;
mod done;
mod feed;
mod kept;
mod leave;
mod prepare;
mod recovery_off;
mod release;
mod save;
mod save_view;
mod saved;

pub(crate) use self::leave::{CloseThisTab, Leaving};
#[cfg(test)]
pub(super) use self::recovery_off::{CONTINUE, message as recovery_off_message};

use futures::channel::oneshot;
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Styled, Subscription, TestSupportExt, Window, component::WindowExt, div,
};
use manyhands::{
    drafts::{DraftBase, DraftErrorKind, DraftFields, DraftId, ItemKind},
    repository::ItemDtoKind,
    results::ResultCode,
};
use zorite_editor::EditorState;

use super::ItemPane;
use crate::desktop::{
    model::{
        edit::{Phase, base_for_edit, edit_offered, fields_with_body, new_item_base},
        links::LinkTarget,
        stamp::ViewScope,
        tabs::TabKey,
    },
    services::requests::DesktopRequestId,
    view::editor::{
        host::{EditorHost, EditorHostEvent},
        toolbar::Toolbar,
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
    NotSentAgain,
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
    #[cfg(test)]
    pub(super) fn base(&self) -> &DraftBase {
        &self.base
    }

    /// Makes the session's base a new item's (R-20): no observation and no
    /// source, as Task 22's new-item drafts are.
    #[cfg(test)]
    pub(super) fn as_new_item(&mut self) {
        self.base.observation = None;
        self.base.source = None;
    }

    /// The session's fields now: the editor's text over the fields besides
    /// it.
    fn current(&self, cx: &App) -> DraftFields {
        fields_with_body(&self.meta, self.host.read(cx).text(cx))
    }

    /// Whether the item was never created: it is dirty until its create
    /// succeeds (R-73).
    fn never_created(&self) -> bool {
        new_item_base(&self.base)
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
    /// Whether the kept draft's item changed, as last classified: for the
    /// draft and revision it was, until a read lands.
    kept_changed: Option<(DraftId, u64, bool)>,
    /// A Discard waiting for the draft still beginning (`leave`).
    discard_begun: Option<oneshot::Sender<Result<(), DraftErrorKind>>>,
    save: save::TabSave,
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
            kept_changed: None,
            discard_begun: None,
            save: save::TabSave::new(scope),
        }
    }

    pub(super) fn phase(&self) -> Phase {
        self.phase
    }

    /// A read landed: while Loading, and not waiting for Resume, the editor
    /// opens from it; a kept draft is classified against it again.
    pub(super) fn read_landed(&mut self) {
        if self.phase == Phase::Loading && self.stopped.is_none() {
            self.loaded = true;
        }
        self.kept_changed = None;
    }
}

/// The store's item kind of a desktop item.
fn item_kind(kind: ItemDtoKind) -> ItemKind {
    match kind {
        ItemDtoKind::Document => ItemKind::Document,
        ItemDtoKind::Ticket => ItemKind::Ticket,
    }
}

impl ItemPane {
    /// The edit session, while one lives, kept or not.
    #[cfg(test)]
    pub(super) fn edit_session(&self) -> Option<&EditSession> {
        self.edit.session.as_ref()
    }

    /// The edit session, for a test to mark it.
    #[cfg(test)]
    pub(super) fn edit_session_mut(&mut self) -> Option<&mut EditSession> {
        self.edit.session.as_mut()
    }

    /// The edit editor's host, while a session lives, kept or not.
    #[cfg(test)]
    pub(crate) fn edit_host(&self) -> Option<Entity<EditorHost>> {
        Some(self.edit.session.as_ref()?.host.clone())
    }

    /// Where the tab stands.
    #[cfg(test)]
    pub(crate) fn edit_phase(&self) -> Phase {
        self.edit.phase
    }

    /// The edit editor's toolbar, while a session lives.
    #[cfg(test)]
    pub(crate) fn edit_toolbar(&self) -> Option<Entity<Toolbar>> {
        Some(self.edit.session.as_ref()?.toolbar.clone())
    }

    /// The edit editor, while it is shown.
    pub(super) fn edit_editor(&self, cx: &App) -> Option<Entity<EditorState>> {
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
    pub(super) fn has_session(&self) -> bool {
        self.edit.session.is_some()
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
        let tab = self.tab_key();
        let TabKey::Item { repo, kind, id } = &tab else {
            return false;
        };
        self.edit.discarded = None;
        let registry = self.services.drafts().read(cx);
        // R-35: a stored draft is continued, not a second one begun. Until
        // Task 18's restored-draft route, Continue editing restores it as
        // D2's banner does.
        if let Some(stored) = registry.stored_for(repo, *kind, id) {
            self.restore(stored, cx);
            return true;
        }
        match registry.unavailable() {
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

    /// Whether Edit can start now: viewing an item it is offered for, with
    /// no draft restored in the tab, which Task 18 continues; a kept
    /// session is continued. A prepare waiting for Resume is in Loading.
    fn edit_available(&self, cx: &App) -> bool {
        let Some(item) = self.model.item().rows() else {
            return false;
        };
        let kept = self.session_kept();
        self.edit.phase == Phase::Viewing
            && edit_offered(self.model.target(), item)
            && (kept
                || self
                    .services
                    .drafts()
                    .read(cx)
                    .attached(&self.tab_key())
                    .is_none())
    }

    /// As a frame is drawn, before the read editor takes the read: the
    /// read that landed while Loading opens the edit editor. A read that
    /// failed, or kept an older one, leaves the tab viewing, with D1's
    /// banners.
    pub(super) fn sync_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.edit.phase != Phase::Loading || !self.edit.loaded || self.edit.stopped.is_some() {
            return;
        }
        self.edit.loaded = false;
        self.edit.phase = Phase::Viewing;
        if self.model.not_refreshed().is_some() {
            return;
        }
        let Some(item) = self.model.item().rows().cloned() else {
            return;
        };
        if !edit_offered(self.model.target(), &item) {
            return;
        }
        let (Some(base), Some(body)) = (base_for_edit(&item), item.body.as_deref()) else {
            return;
        };
        self.open_editor(base, body, window, cx);
    }

    /// Opens the edit editor on `body`, the read's, whose loaded text is
    /// `base`'s body. When the read editor holds that same text, its caret
    /// carries across, clamped (R-65); otherwise the read editor reloads
    /// from this read as the frame goes on, and the caret starts at the
    /// start.
    fn open_editor(
        &mut self,
        base: DraftBase,
        body: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let loaded = match &base.fields {
            DraftFields::Document { body, .. } | DraftFields::Ticket { body, .. } => body.as_str(),
            DraftFields::Comment { .. } => return,
        };
        let cursor = self.page.read_cursor_if_holding(loaded, cx);
        let host = cx.new(|cx| EditorHost::new(body, cursor, window, cx));
        let (done, scroll) = (self.edit.done.clone(), self.scroll.clone());
        host.update(cx, |host, cx| {
            host.place(done, &scroll, window, cx);
            host.show_images(&self.images, cx);
        });
        let toolbar = cx.new(|cx| Toolbar::new(host.clone(), window, cx));
        let subscriptions = [
            cx.subscribe(&host, |pane, _, event: &EditorHostEvent, cx| match event {
                EditorHostEvent::Edited => pane.feed(cx),
                EditorHostEvent::DirtyChanged(_) => cx.notify(),
                EditorHostEvent::SelectionChanged | EditorHostEvent::LinkRequested => {}
            }),
            // Decision 10: the edit editor's links go where the tab's others
            // do.
            cx.subscribe(&host, |pane, _, target: &LinkTarget, cx| {
                pane.follow_link(target, cx)
            }),
        ];
        let editor = host.read(cx).editor().clone();
        let meta = base.fields.clone();
        self.edit.session = Some(EditSession {
            host,
            toolbar,
            base,
            meta,
            draft: DraftLink::None,
            saved: false,
            dirty_seen: false,
            kept: false,
            _subscriptions: subscriptions,
        });
        self.edit.phase = Phase::Editing;
        self.sync_status(cx);
        // Focus moves into the editor once the frame that draws it is
        // drawn.
        cx.defer_in(window, move |_, window, cx| {
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        });
        cx.notify();
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
    /// switches. False when no editor is shown, so the key goes on.
    pub(super) fn toggle_rich(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = self.shown_session() else {
            return false;
        };
        session
            .host
            .update(cx, |host, cx| host.set_rich(!host.is_rich(), cx));
        true
    }

    /// The toolbar, under the header while editing.
    pub(super) fn toolbar_view(&self) -> Option<AnyElement> {
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

    /// The page's edit editor, while editing; a kept session's page is the
    /// saved copy's.
    pub(super) fn edit_view(&self) -> Option<AnyElement> {
        let session = self.shown_session()?;
        Some(self.page.edit_view(&session.host))
    }
}
