//! Late results for the edit editor (design, Late results; contract 6;
//! R-19, R-70). Every read applies under its tab's stamp, so a read sent
//! before Loading's or a save's own read never lands; the read after a
//! committed save is `saved`'s. Of any other read that lands:
//!
//! - a dirty edit editor, one whose draft is live, one kept after Done, a
//!   never-created item's and one whose save runs are left alone: their
//!   text is the user's (Task 16 fix round 2's rule, for every read);
//! - a clean one takes a read whose observation differs from its base's.
//!   Clean is the session's, against its saved copy: the editor's own flag
//!   can still read dirty after a save that newer typing outlived, and it
//!   does not decide (Task 18 fix 1). The editor loads only a body that
//!   differs from what it holds, keeping its caret, clamped
//!   (`EditorHost::reload_read`); an equal body is not loaded again, so
//!   undo is kept. Only once the editor holds the read's body does a
//!   verified base become the read's, so the session stays clean on it
//!   and a later Save never sends our older text over another actor's
//!   token (contract 7). An unverified base stays, and the next Save opens
//!   the review (R-36).

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
        let holds = session
            .host
            .update(cx, |host, cx| host.reload_read(&body, cx));
        if holds && session.base.observation.is_some() {
            session.meta = base.fields.clone();
            session.base = base;
        }
        cx.notify();
    }
}
