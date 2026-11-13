//! Late results for the edit editor (design, Late results; contract 6;
//! R-19, R-70). Every read applies under its tab's stamp, so a read sent
//! before Loading's or a save's own read never lands; the read after a
//! committed save is `saved`'s, which takes it by the same rule. Of any
//! other read that lands:
//!
//! - a dirty edit editor, one whose draft is live, one kept after Done, a
//!   never-created item's, one whose save runs and one under the review
//!   are left alone: their text is the user's (Task 16 fix round 2's rule,
//!   for every read);
//! - a clean one takes a read whose observation differs from its base's
//!   (`take_clean_read`). Clean is the session's, against its saved copy:
//!   the editor's own flag can still read dirty after a save that newer
//!   typing outlived, and it does not decide (Task 18 fix 1). The editor
//!   loads only a body that differs from what it holds, keeping its caret,
//!   clamped (`EditorHost::reload_read`); a body it holds already is not
//!   loaded again, so undo is kept. Only once the editor holds the read's
//!   body does the base move:
//!   - a verified base becomes the read's, so the session stays clean on
//!     it and a later Save never sends our older text over another actor's
//!     token (contract 7);
//!   - an unverified base takes the body the editor now holds and stays
//!     unverified (Task 19's decision on Task 16's N2 and Task 18's M3): the
//!     session stays clean on what it shows, and the next Save opens the
//!     review (R-36), which then compares the user's typing with their
//!     body alone.

use gpui_kit::Context;
use manyhands::repository::ItemDto;

use super::Phase;
use crate::desktop::{
    model::edit::{base_for_edit, fields_with_body},
    view::item::ItemPane,
};

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
        let item = item.clone();
        self.take_clean_read(&item, cx);
    }

    /// A clean session, with no draft, takes `item`, a read of it: the
    /// editor holds its body, loaded once when it differs (R-19, R-79), and
    /// then the base moves, by the rule above.
    pub(super) fn take_clean_read(&mut self, item: &ItemDto, cx: &mut Context<Self>) {
        let (Some(read), Some(body)) = (base_for_edit(item), item.body.clone()) else {
            return;
        };
        let Some(session) = self.edit.session.as_mut() else {
            return;
        };
        let holds = session
            .host
            .update(cx, |host, cx| host.reload_read(&body, cx));
        if !holds {
            cx.notify();
            return;
        }
        // The body as the editor holds it, which the base then holds too.
        let text = session.host.read(cx).text(cx);
        if session.base.observation.is_some() {
            session.base = read;
            session.base.fields = fields_with_body(&session.base.fields, text);
            session.meta = session.base.fields.clone();
        } else {
            session.base.fields = fields_with_body(&session.base.fields, text);
        }
        cx.notify();
    }
}
