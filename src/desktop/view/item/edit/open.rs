//! The edit editor opened (design, Part C, Phases, step 3; Restored drafts
//! and the close dialog): as a frame is drawn, the read that landed while
//! Loading opens it, on the item's body for an Edit, or on a continued
//! draft's text (`restored`), with no item read for a new item's draft
//! whose item is not created yet (`create`).

use gpui_kit::{AppContext, Context, Window};
use manyhands::drafts::{DraftBase, DraftFields};

use super::{DraftLink, EditSession, Phase};
use crate::desktop::{
    model::{
        edit::{base_for_edit, edit_offered},
        links::LinkTarget,
    },
    view::{
        editor::{
            host::{EditorHost, EditorHostEvent},
            toolbar::Toolbar,
        },
        item::ItemPane,
    },
};

/// What a session opens with: its base, the fields besides the body, and
/// its draft. It holds draft text, so it derives nothing.
pub(super) struct Opening {
    pub(super) base: DraftBase,
    pub(super) meta: DraftFields,
    pub(super) draft: DraftLink,
}

/// The edit editor's text: the body as loaded, the caret at the read
/// editor's when it carries across; or a continued draft's, exactly.
pub(super) enum Text<'a> {
    Read(Option<usize>),
    Draft(&'a str),
}

impl ItemPane {
    /// As a frame is drawn, before the read editor takes the read: the
    /// read that landed while Loading opens the edit editor, on a continued
    /// draft when Continue editing asked for one. A read that failed, or
    /// kept an older one, leaves the tab viewing, with D1's banners; a
    /// continued draft stays where it was, restored in the tab.
    pub(in crate::desktop::view::item) fn sync_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.edit.phase != Phase::Loading || !self.edit.loaded || self.edit.stopped.is_some() {
            return;
        }
        self.edit.loaded = false;
        self.edit.phase = Phase::Viewing;
        let continuing = self.edit.continuing.take();
        // A new item's draft prepared with Create opens by what this read
        // says of its item (R-42): with none to read, on the draft alone.
        let continuing = match continuing {
            Some(continuing) if continuing.create => {
                return self.open_not_created(continuing, window, cx);
            }
            continuing => continuing,
        };
        if self.model.not_refreshed().is_some() {
            return;
        }
        let Some(item) = self.model.item().rows().cloned() else {
            return;
        };
        if !edit_offered(self.model.target(), &item) {
            return;
        }
        if let Some(continuing) = continuing {
            self.open_restored(continuing, &item, window, cx);
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
        let opening = Opening {
            meta: base.fields.clone(),
            base,
            draft: DraftLink::None,
        };
        self.open_session(opening, body, Text::Read(cursor), window, cx);
    }

    /// Opens the edit editor on `body`, the read's, loaded once, with
    /// `text`: the caret where it says, or a continued draft's text put in
    /// exactly over the loaded body (R-79), the caret at the start. The
    /// session holds what `opening` gives. The toolbar appears, and focus
    /// moves into the editor.
    pub(super) fn open_session(
        &mut self,
        opening: Opening,
        body: &str,
        text: Text<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cursor = match text {
            Text::Read(cursor) => cursor,
            Text::Draft(_) => None,
        };
        let host = cx.new(|cx| EditorHost::new(body, cursor, window, cx));
        let (done, scroll) = (self.edit.done.clone(), self.scroll.clone());
        host.update(cx, |host, cx| {
            host.place(done, &scroll, window, cx);
            host.show_images(&self.images, cx);
            if let Text::Draft(text) = text {
                host.put_exact(text, 0, cx);
            }
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
        let Opening { base, meta, draft } = opening;
        let mut session = EditSession {
            host,
            toolbar,
            base,
            meta,
            draft,
            saved: false,
            dirty_seen: false,
            kept: false,
            _subscriptions: subscriptions,
        };
        session.dirty_seen = session.dirty(cx);
        self.edit.session = Some(session);
        self.edit.phase = Phase::Editing;
        self.sync_status(cx);
        // Nothing an earlier session was answered holds; a continued
        // draft's relationships are checked.
        self.relations_opened(cx);
        // Focus moves into the editor once the frame that draws it is
        // drawn.
        cx.defer_in(window, move |_, window, cx| {
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        });
        cx.notify();
    }
}
