//! A new item's draft in its tab (design, Create (decision 3); Restored
//! drafts and the close dialog; R-20, R-42, R-73): routed by its
//! classification, continued with no item read, created by Save, and
//! reviewed when its item turns out to exist.
//!
//! - **The route** (R-42). A new item's draft, stored or restored here,
//!   whose item the tab's read finds missing classifies `NotCreated`: Edit
//!   and Continue editing are offered with no item read, and its `context
//!   prepare` takes the Create intent. Any other draft takes the Edit
//!   intent (`restored`).
//! - **A Create answering `occupied_path`**: the index has caught up with
//!   an item that holds the ID. The draft takes the Edit route instead, and
//!   the review opens on the item read again (`restored`).
//! - **Opening.** Once the Create prepare's read lands, a draft still
//!   `NotCreated` opens the edit editor on its own text, exactly (R-79),
//!   over its own base, with no item read; one whose item that read finds
//!   goes the Edit route's way, unverified, to the review. The session is
//!   dirty until its create succeeds (R-73), so Save is offered for a
//!   titled item with an empty body.
//! - **Save** is the registry's `create` (`save`), under Save's one gate
//!   (`save_blocked`): a new ticket with an empty Title, Type or Status, or
//!   a held relationship rejection, is not created. Its completion is a
//!   save's (`saved`): the base the registry read clears the marker.
//! - **`occupied_path` for its own item.** Once the registry has read the
//!   path and found this draft's item there (an earlier create's answer
//!   was lost), the session goes on over the registry's unverified base,
//!   and the review opens (`follow_unmarked`).
//!
//! - **A create dialog's new item** (R-20; `view::create`). Its draft is
//!   written, and its Create prepare sent, before its tab opens: the tab
//!   takes the draft and follows that prepare as Continue editing does, to
//!   the same opening (`edit_created`).
//! - **The header** (design, Create: the new tab opens editing, with its
//!   header built from the draft's fields). With no item to read, the tab
//!   shows the draft's fields as its item, in the header, the strip and the
//!   sheet, while the never-created session lives and while its Create
//!   prepare runs (`new_item_shown`); D1's not-found line and D2's restored
//!   section are not shown meanwhile.
//!
//! Done, Discard and the Drafts page's discard of such a session are
//! `done` and `leave`'s: there is no saved copy to view, so the tab
//! closes; release on leave releases only a context this process made
//! (decision 18). Nothing here holds draft text but the session it opens.

use gpui_kit::{App, Context, Window};
use manyhands::{
    drafts::{TargetState, classify, is_new_item_base},
    repository::{ContextPrepareIntent, ItemDto},
    results::ResultCode,
};

use super::{
    DraftLink, Phase,
    open::{Opening, Text},
    restored::{Continued, Continuing, Then},
    saved::body_of,
};
use crate::desktop::{
    model::{drafts::create::shown_new_item, edit::edit_offered, navigation::OpenTarget},
    services::drafts::NewItem,
    view::item::ItemPane,
};

impl ItemPane {
    /// A create dialog's new item (R-20): its draft, written before its
    /// Create prepare was sent, is attached to this, its tab, and the tab
    /// follows that prepare as Continue editing does: once its read lands,
    /// the edit editor opens on the draft (`open_not_created`).
    pub(in crate::desktop::view) fn edit_created(
        &mut self,
        created: NewItem,
        cx: &mut Context<Self>,
    ) {
        let tab = self.tab_key();
        self.services
            .drafts()
            .update(cx, |registry, cx| registry.attach(tab, created.draft, cx));
        self.edit.continuing = Some(Continuing {
            draft: created.draft,
            then: Then::Edit,
            create: true,
        });
        self.edit.phase = Phase::Preparing(created.prepare);
        self.edit.prepare_taken = 0;
        self.edit.stopped = None;
        // The prepare may have been answered before the tab was there.
        self.prepare_progressed(cx);
        cx.notify();
    }

    /// The new item this tab shows while it is not created yet: with no
    /// item read, the never-created session's fields, or, while a Create
    /// prepare runs for the draft continued here, that draft's. `None`
    /// otherwise, a stored or restored draft not continued included.
    pub(in crate::desktop::view) fn new_item_shown(&self, cx: &App) -> Option<ItemDto> {
        if self.model.item().rows().is_some() {
            return None;
        }
        let OpenTarget::Item { kind, id, .. } = self.model.target() else {
            return None;
        };
        if let Some(session) = &self.edit.session {
            return session
                .never_created()
                .then(|| shown_new_item(*kind, id, &session.base, &session.meta));
        }
        if !self.opening_new_item() {
            return None;
        }
        let continuing = self.edit.continuing.as_ref()?;
        let registry = self.services.drafts().read(cx);
        let session = registry.session(registry.current_id(continuing.draft)?)?;
        Some(shown_new_item(
            *kind,
            id,
            session.base()?,
            session.current(),
        ))
    }

    /// Whether the Create prepare of a draft continued here runs: the tab
    /// is opening its editor on a new item's draft.
    pub(in crate::desktop::view::item) fn opening_new_item(&self) -> bool {
        matches!(self.edit.phase, Phase::Preparing(_) | Phase::Loading)
            && self
                .edit
                .continuing
                .as_ref()
                .is_some_and(|continuing| continuing.create)
    }

    /// Whether the draft Continue editing would continue is a new item's
    /// whose item the tab's read finds missing (`NotCreated`): it is
    /// continued with the Create intent, and with no item read (R-42).
    pub(in crate::desktop::view::item) fn continued_not_created(&self, cx: &App) -> bool {
        let Some(Err(code)) = self.model.item_read() else {
            return false;
        };
        let registry = self.services.drafts().read(cx);
        let state = match self.continued(cx) {
            Some(Continued::Restored(id)) => registry.classify(id, Err(code)),
            Some(Continued::Stored(id)) => registry
                .listed(id)
                .map(|(draft, _)| classify(draft, Err(code))),
            None => None,
        };
        matches!(state, Some(TargetState::NotCreated))
    }

    /// Prepares the context of the draft Continue editing continues, with
    /// the Create intent for a new item's draft not created yet, and
    /// otherwise the Edit intent (R-42). A prepare that did not start
    /// continues nothing.
    pub(super) fn prepare_continued(&mut self, cx: &mut Context<Self>) {
        let create = self
            .edit
            .continuing
            .as_ref()
            .is_some_and(|continuing| continuing.create);
        let intent = match create {
            true => ContextPrepareIntent::Create,
            false => ContextPrepareIntent::Edit,
        };
        self.prepare_with(intent, cx);
        if !matches!(self.edit.phase, Phase::Preparing(_)) {
            self.edit.continuing = None;
        }
    }

    /// R-42: a Create prepare answered `occupied_path`, an item holding the
    /// ID now (the index caught up with it). The draft takes the Edit
    /// route, and the review. False for any other answer.
    pub(super) fn create_rerouted(&mut self, code: ResultCode, cx: &mut Context<Self>) -> bool {
        let Some(continuing) = &mut self.edit.continuing else {
            return false;
        };
        if code != ResultCode::OccupiedPath || !continuing.create {
            return false;
        }
        continuing.create = false;
        self.edit.phase = Phase::Viewing;
        self.prepare_continued(cx);
        true
    }

    /// The read after a Create prepare landed for the continued draft. Still
    /// `NotCreated`, the edit editor opens on the draft's own text and base,
    /// with no item read; once its item exists, the Edit route's way, to the
    /// review (R-42). A draft gone meanwhile, and a read that failed
    /// otherwise, open nothing: the draft stays restored in the tab.
    pub(super) fn open_not_created(
        &mut self,
        continuing: Continuing,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let opening = {
            let registry = self.services.drafts().read(cx);
            let Some(id) = registry.current_id(continuing.draft) else {
                return;
            };
            let Some(read) = self.model.item_read() else {
                return;
            };
            match registry.classify(id, read) {
                Some(TargetState::NotCreated) => registry.session(id).and_then(|session| {
                    let base = session.base()?.clone();
                    let meta = session.current().clone();
                    Some((id, base, meta))
                }),
                _ => None,
            }
        };
        let Some((id, base, meta)) = opening else {
            // Its item exists after all.
            if self.model.not_refreshed().is_some() {
                return;
            }
            let Some(item) = self.model.item().rows().cloned() else {
                return;
            };
            if edit_offered(self.model.target(), &item) {
                self.open_restored(continuing, &item, window, cx);
            }
            return;
        };
        let loaded = body_of(&base.fields).to_owned();
        let text = body_of(&meta).to_owned();
        let opening = Opening {
            base,
            meta,
            draft: DraftLink::Live(id),
        };
        self.open_session(opening, &loaded, Text::Draft(&text), window, cx);
        if continuing.then == Then::Save {
            // Once the frame that opened the editor is drawn.
            cx.defer_in(window, |pane, _, cx| {
                pane.start_save(cx);
            });
        }
    }

    /// The registry found this never-created session's own item at its
    /// path (a create's `occupied_path`, an earlier answer lost) and moved
    /// its draft onto an ordinary unverified base: the session goes on over
    /// it, the refusal is the review's now, and the review opens (R-42).
    pub(in crate::desktop::view::item) fn follow_unmarked(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.shown_session() else {
            return;
        };
        if !session.never_created()
            || self.edit.phase != Phase::Editing
            || self.edit.save.run.is_some()
        {
            return;
        }
        let Some(id) = self.live_draft(cx) else {
            return;
        };
        let base = self
            .services
            .drafts()
            .read(cx)
            .session(id)
            .and_then(|session| session.base())
            .filter(|base| !is_new_item_base(base))
            .cloned();
        let (Some(base), Some(session)) = (base, &mut self.edit.session) else {
            return;
        };
        session.base = base;
        let occupied = self
            .edit
            .save
            .answer
            .as_ref()
            .is_some_and(|answer| answer.code == ResultCode::OccupiedPath);
        if occupied {
            self.edit.save.answer = None;
        }
        self.open_review(cx);
    }
}
