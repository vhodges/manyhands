//! Late results for the edit editor (design, Late results; contract 6;
//! R-19, R-70). Every read applies under its tab's stamp, so a read sent
//! before Loading's or a save's own read never lands; the read after a
//! committed save is `saved`'s. Of any other read that lands:
//!
//! - a dirty edit editor, one whose draft is live, one kept after Done, a
//!   never-created item's and one whose save runs are left alone: their
//!   text is the user's (Task 16 fix round 2's rule, for every read);
//! - a clean one takes a read whose observation differs from its base's.
//!   It reloads only a body that differs from what it holds, keeping its
//!   caret, clamped (`EditorHost::reload_clean`); an equal body is not
//!   loaded again, so undo is kept. A verified base becomes the read's, so
//!   the session stays clean on it; an unverified one stays, and the next
//!   Save opens the review (R-36).

use gpui_kit::Context;

use super::Phase;
use crate::desktop::{model::edit::base_for_edit, view::item::ItemPane};

impl ItemPane {
    /// A read landed, not the tab's own read after a save: a clean edit
    /// editor takes it.
    pub(in crate::desktop::view::item) fn late_read_landed(&mut self, cx: &mut Context<Self>) {
        if self.edit.phase != Phase::Editing
            || self.edit.save.run.is_some()
            || self.store_settling(cx)
            || self.live_draft(cx).is_some()
        {
            return;
        }
        let Some(session) = self.shown_session() else {
            return;
        };
        if session.dirty(cx) || session.never_created() {
            return;
        }
        let Some(item) = self.model.item().rows() else {
            return;
        };
        if item.observation.is_some() && item.observation == session.base.observation {
            return;
        }
        let (Some(base), Some(body)) = (base_for_edit(item), item.body.clone()) else {
            return;
        };
        let Some(session) = self.edit.session.as_mut() else {
            return;
        };
        if session.base.observation.is_some() {
            session.meta = base.fields.clone();
            session.base = base;
        }
        session
            .host
            .update(cx, |host, cx| host.reload_clean(&body, cx));
        cx.notify();
    }
}
