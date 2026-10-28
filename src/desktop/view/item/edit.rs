//! Edit (design, Part C, Phases, steps 1 to 4): the item's private work
//! context made ready, the edit editor opened on a fresh read, and D2's
//! draft begun and fed as the text changes.
//!
//! - **Viewing.** Edit, `e` on the page and `secondary-shift-e` in the tab
//!   start it, where it is offered (`model::edit::edit_offered`). While a
//!   draft of the item is kept or stored it reads Continue editing (R-35).
//! - **Preparing.** `context prepare` with the Edit intent runs on the user
//!   lane. The pill says the private copy is being made ready, Edit is
//!   disabled, and the page stays readable. A tab closed meanwhile opens
//!   nothing; a context the answer made is still counted, process-wide
//!   (`services::created_contexts`).
//! - **Loading.** On `ok` the item is read again under a new stamp (R-70),
//!   and the edit editor opens from that read: on the body as the editor
//!   loads it, with the read editor's caret when the read editor holds that
//!   text (R-65), the toolbar above the page and focus in the editor. A
//!   prepare that answered `discovery_pending` waits in Loading for Resume,
//!   which sends the same request again (R-13, R-77): the pill still says
//!   the copy is being made ready, and no read opens the editor meanwhile.
//!   A refusal is shown in a banner, and the tab stays viewing.
//! - **Editing.** The first difference from the base begins D2's draft,
//!   on the base the read gave (R-11); text typed before it lands is
//!   written with it, and every later difference, undo's included (R-12),
//!   updates it. The draft is attached to the tab. A draft store that did
//!   not open is said before editing starts, and the user chooses whether
//!   to go on (`recovery_off`). The toolbar's status item says what D2
//!   holds of the text.
//!
//! Save is Task 15's, Done and the kept session Task 16's, and release on
//! leave Task 17's. Draft text stays in the editor, the registry and the
//! store; nothing here derives `Debug`.

mod actions;
mod feed;
mod recovery_off;

#[cfg(test)]
pub(super) use self::recovery_off::{CONTINUE, message as recovery_off_message};

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Styled, Subscription, TestSupportExt, Window, component::WindowExt, div,
};
use manyhands::{
    drafts::{DraftBase, DraftFields, DraftId, ItemKind},
    repository::{AuthoringKind, ContextPrepareIntent, ItemDtoKind},
    results::{RecoveryActionKind, ResultCode},
};
use zorite_editor::EditorState;

use super::ItemPane;
use crate::desktop::{
    model::{
        edit::{Phase, base_for_edit, edit_offered},
        links::LinkTarget,
        navigation::OpenTarget,
        tabs::TabKey,
    },
    services::requests::{DesktopRequestId, Intent, Lane, RequestState},
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
pub(crate) const RESUME: &str = "Resume";
pub(crate) const RETRY: &str = "Retry";

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
/// begins on, and the draft. It holds the user's text, so it derives
/// nothing.
pub(super) struct EditSession {
    host: Entity<EditorHost>,
    toolbar: Entity<Toolbar>,
    /// The item as read when the editor opened, normalized (R-11).
    base: DraftBase,
    draft: DraftLink,
    /// A save has committed in this session.
    saved: bool,
    /// The session is kept after Done.
    kept: bool,
    _subscriptions: [Subscription; 2],
}

impl EditSession {
    /// Whether a save has committed in this session.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 15.
    pub(super) fn saved(&self) -> bool {
        self.saved
    }

    /// Whether the session is kept after Done.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 16.
    pub(super) fn kept(&self) -> bool {
        self.kept
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
}

impl TabEdit {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            phase: Phase::Viewing,
            loaded: false,
            stopped: None,
            session: None,
            done: cx.focus_handle().tab_stop(true),
        }
    }

    pub(super) fn phase(&self) -> Phase {
        self.phase
    }

    /// A read landed: while Loading, and not waiting for Resume, the editor
    /// opens from it.
    pub(super) fn read_landed(&mut self) {
        if self.phase == Phase::Loading && self.stopped.is_none() {
            self.loaded = true;
        }
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
    /// The edit session, while one lives.
    #[cfg(test)]
    pub(super) fn edit_session(&self) -> Option<&EditSession> {
        self.edit.session.as_ref()
    }

    /// The edit editor's host, while a session lives.
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

    /// The edit editor, while a session lives.
    pub(super) fn edit_editor(&self, cx: &App) -> Option<Entity<EditorState>> {
        Some(self.edit.session.as_ref()?.host.read(cx).editor().clone())
    }

    /// Whether the edit editor is shown: Editing, Saving or the Review.
    pub(super) fn editing(&self) -> bool {
        self.edit.session.is_some()
    }

    /// Whether the edit editor's text differs from its baseline.
    pub(super) fn edit_dirty(&self, cx: &App) -> bool {
        self.edit
            .session
            .as_ref()
            .is_some_and(|session| session.host.read(cx).is_dirty())
    }

    /// Edit, Continue editing, `e` and `secondary-shift-e`. False when
    /// nothing was started, so a key goes on.
    pub(super) fn edit_requested(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // A dialog in front of the tab takes every key (controller ruling,
        // 2026-10-27).
        if window.has_active_dialog(cx) || !self.edit_available(cx) {
            return false;
        }
        let tab = self.tab_key();
        let TabKey::Item { repo, kind, id } = &tab else {
            return false;
        };
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
    /// no draft restored in the tab, which Task 18 continues. A prepare
    /// waiting for Resume is in Loading.
    fn edit_available(&self, cx: &App) -> bool {
        let Some(item) = self.model.item().rows() else {
            return false;
        };
        self.edit.phase == Phase::Viewing
            && edit_offered(self.model.target(), item)
            && self
                .services
                .drafts()
                .read(cx)
                .attached(&self.tab_key())
                .is_none()
    }

    /// Submits `context prepare` with the Edit intent for this tab's item.
    fn prepare(&mut self, cx: &mut Context<Self>) {
        let OpenTarget::Item { repo, kind, id } = self.model.target().clone() else {
            return;
        };
        if self.edit.phase != Phase::Viewing {
            return;
        }
        let kind = match kind {
            ItemDtoKind::Document => AuthoringKind::Document,
            ItemDtoKind::Ticket => AuthoringKind::Ticket,
        };
        let request = self.services.worker().submit(
            Intent::ContextPrepare {
                root: PathBuf::from(repo),
                kind,
                id,
                intent: ContextPrepareIntent::Edit,
            },
            Lane::User,
            None,
            None,
        );
        self.edit.phase = Phase::Preparing(request);
        self.edit.stopped = None;
        cx.notify();
    }

    /// Resume of a prepare whose hand-off is owed: the same request again,
    /// under its own request ID (R-77).
    fn resume(&mut self, cx: &mut Context<Self>) {
        let Some(Stopped::Pending(request)) = self.edit.stopped else {
            return;
        };
        self.edit.stopped = None;
        self.edit.phase = Phase::Preparing(request);
        self.services.worker().retry(request, None);
        cx.notify();
    }

    /// The prepare's request moved: on `ok` the item is read again under a
    /// new stamp (R-70); `discovery_pending` waits for Resume; anything
    /// else is a refusal, shown, and the tab stays viewing.
    pub(super) fn prepare_progressed(&mut self, cx: &mut Context<Self>) {
        let Phase::Preparing(request) = self.edit.phase else {
            return;
        };
        let state = self
            .services
            .operations()
            .read(cx)
            .request(request)
            .map(|request| request.state.clone());
        match state {
            None | Some(RequestState::Queued | RequestState::Running { .. }) => return,
            Some(RequestState::Finished(_, ResultCode::Ok, ..)) => {
                self.edit.phase = Phase::Loading;
                self.edit.loaded = false;
                self.reload(cx);
            }
            // R-13, R-77: editing waits in Loading for Resume (controller
            // ruling, 2026-10-28). No read opens the editor meanwhile.
            Some(RequestState::Finished(_, ResultCode::DiscoveryPending, ..)) => {
                self.edit.phase = Phase::Loading;
                self.edit.loaded = false;
                self.edit.stopped = Some(Stopped::Pending(request));
            }
            Some(RequestState::Finished(_, code, _, recovery, message)) => {
                let retry = recovery
                    .iter()
                    .any(|action| action.action == RecoveryActionKind::RequestRetry)
                    .then_some(request);
                self.edit.phase = Phase::Viewing;
                self.edit.stopped = Some(Stopped::Refused {
                    code,
                    message,
                    retry,
                });
            }
            Some(RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_)) => {
                self.edit.phase = Phase::Viewing;
            }
        }
        cx.notify();
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
        self.edit.session = Some(EditSession {
            host,
            toolbar,
            base,
            draft: DraftLink::None,
            saved: false,
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

    /// The mode key, from anywhere in the tab while editing: the host
    /// switches. False when there is no editor, so the key goes on.
    pub(super) fn toggle_rich(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = &self.edit.session else {
            return false;
        };
        session
            .host
            .update(cx, |host, cx| host.set_rich(!host.is_rich(), cx));
        true
    }

    /// The toolbar, under the header while editing.
    pub(super) fn toolbar_view(&self) -> Option<AnyElement> {
        let session = self.edit.session.as_ref()?;
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

    /// The page's edit editor, while editing.
    pub(super) fn edit_view(&self) -> Option<AnyElement> {
        let session = self.edit.session.as_ref()?;
        Some(self.page.edit_view(&session.host))
    }
}
