//! The session after a committed save (design, Part C, Phases, step 6;
//! R-69, R-70): on D2's `DraftSaved`, or on this tab's own read when no D2
//! session held the text, the session has saved, its baseline and base
//! move, the tab reads the item again, and the remotes are read (fact 41).
//! A clean session takes the tab's read after the save as a late read is
//! taken (`late`), once D2 has retired the saved draft
//! (`follow_post_save_read`). The tab's save also follows what D2 does
//! with it (`follow_registry`). The save itself is `save`.

use std::path::PathBuf;

use gpui_kit::Context;
use manyhands::{
    drafts::{DraftBase, DraftFields},
    repository::ItemDto,
    results::ResultCode,
};

use crate::desktop::{
    model::{
        drafts::{Saving, save::base_after_save},
        edit::Phase,
        navigation::OpenTarget,
        stamp::Stamp,
    },
    services::{ReadKind, ReadResult, drafts::DraftSaved},
    view::item::ItemPane,
};

/// The body among `fields`.
pub(super) fn body_of(fields: &DraftFields) -> &str {
    match fields {
        DraftFields::Document { body, .. }
        | DraftFields::Ticket { body, .. }
        | DraftFields::Comment { body } => body,
    }
}

/// A read, as `base_after_save` takes it.
fn read_item(read: &ReadResult<ItemDto>) -> Result<&ItemDto, ResultCode> {
    match read {
        ReadResult::Ok(item) => Ok(item),
        ReadResult::Busy => Err(ResultCode::Busy),
        ReadResult::Failed(failure) => Err(failure.code),
    }
}

impl ItemPane {
    /// A save with no D2 session committed: the item is read again for the
    /// base it leaves, by D2's rule.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    pub(super) fn read_after_direct(&mut self, submitted: DraftFields, cx: &mut Context<Self>) {
        let OpenTarget::Item { repo, id, .. } = self.model.target().clone() else {
            return;
        };
        let root = PathBuf::from(repo);
        let stamp = self.edit.save.stamp();
        self.edit.save.read = Some(self.services.read(
            ReadKind::DraftItem,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.show_item(&repo, &id)
            },
            cx,
            move |pane: &mut Self, stamp, read, cx| {
                if !pane.edit.save.generations.is_current(&stamp) {
                    return;
                }
                let Some(session) = &pane.edit.session else {
                    return;
                };
                let base = base_after_save(&submitted, read_item(&read), &session.base);
                pane.after_save(&submitted, base, cx);
            },
        ));
    }

    /// D2's `DraftSaved`: for this tab's session, the save settles.
    pub(in crate::desktop::view::item) fn draft_saved(
        &mut self,
        event: &DraftSaved,
        cx: &mut Context<Self>,
    ) {
        if self.live_draft(cx) != Some(event.draft) {
            return;
        }
        // Its answer is taken first, for its banner.
        self.save_progressed(cx);
        self.after_save(&event.submitted, event.base.clone(), cx);
    }

    /// A save of `submitted` committed and leaves `base` (R-69, R-70).
    fn after_save(&mut self, submitted: &DraftFields, base: DraftBase, cx: &mut Context<Self>) {
        let Some(session) = &mut self.edit.session else {
            return;
        };
        session.saved = true;
        if session.current(cx) == *submitted {
            let saved = body_of(submitted);
            session
                .host
                .update(cx, |host, cx| host.mark_saved(saved, cx));
        }
        session.base = base;
        self.edit.save.run = None;
        if self.edit.phase == Phase::Saving {
            self.edit.phase = Phase::Editing;
        }
        self.edit.save.reread = Some(self.reload(cx));
        self.read_remotes(cx);
        // Typed while no D2 session held the text: drafted on the new base.
        if self.live_draft(cx).is_none() {
            self.feed(cx);
        }
        self.sync_status(cx);
        // Done waiting for this save goes on (design, Done and leaving, step
        // 2).
        self.done_saved(cx);
        // The close dialog's Save closes the tab once it has settled.
        self.follow_close_save(cx);
        cx.notify();
    }

    /// Whether a publication remote is selected: read after a save, never
    /// on load (fact 41).
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn read_remotes(&mut self, cx: &mut Context<Self>) {
        let root = PathBuf::from(self.repo());
        let stamp = self.edit.save.stamp();
        self.edit.save.remotes = Some(self.services.read(
            ReadKind::Remotes,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.list_remotes_redacted(&repo)
            },
            cx,
            |pane: &mut Self, stamp, read, cx| {
                if !pane.edit.save.generations.is_current(&stamp) {
                    return;
                }
                if let ReadResult::Ok(list) = read {
                    pane.edit.save.remote = list
                        .items
                        .iter()
                        .any(|remote| remote.selected_for_publication);
                    cx.notify();
                }
            },
        ));
    }

    /// The tab's read after a save landed (R-70): a clean edit editor (not
    /// dirty, no draft; design, Late results) takes it by the late reads'
    /// rule (`take_clean_read`): its body, loaded once only when it differs
    /// from what the editor holds (R-19, R-79), then the base. Clean is the
    /// session's, against the saved copy, never the editor's own flag (Task
    /// 18 fix 1): text undone back past the save, while the read was out,
    /// stays with its undo history. While D2 still retires the saved draft,
    /// the read waits until it has (`follow_post_save_read`). False when
    /// `stamp` is not that read's: another read is `late`'s.
    pub(in crate::desktop::view::item) fn post_save_read_landed(
        &mut self,
        stamp: &Stamp,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.edit.save.reread.as_ref() != Some(stamp) {
            return false;
        }
        self.edit.save.reread = None;
        self.edit.save.reread_waits = true;
        self.follow_post_save_read(cx);
        true
    }

    /// The read after a save, once landed: taken by a clean session whose
    /// draft has ended, as the save retired it. One that is dirty, under
    /// the review, or whose draft goes on, holds the user's text, and the
    /// read is let go of; one whose draft D2 still retires waits for it.
    /// A read that kept older rows is not taken.
    pub(in crate::desktop::view::item) fn follow_post_save_read(&mut self, cx: &mut Context<Self>) {
        if !self.edit.save.reread_waits {
            return;
        }
        let Some(session) = &self.edit.session else {
            self.edit.save.reread_waits = false;
            return;
        };
        if session.dirty(cx) || self.edit.phase == Phase::Review {
            self.edit.save.reread_waits = false;
            return;
        }
        if self.live_draft(cx).is_some() {
            if !self.store_settling(cx) {
                self.edit.save.reread_waits = false;
            }
            return;
        }
        self.edit.save.reread_waits = false;
        if self.model.not_refreshed().is_some() {
            return;
        }
        let Some(item) = self.model.item().rows().cloned() else {
            return;
        };
        self.take_clean_read(&item, cx);
    }

    /// The registry changed, and the tab's save follows D2's:
    /// - a committed save whose session D2 ended before it settled, as a
    ///   discard does, has nothing left to settle it;
    /// - a pending checkpoint D2 no longer holds is over: its operation was
    ///   abandoned, or its session ended. Its banner goes, and Save is
    ///   enabled again, with no Retry bound to fail;
    /// - a save waiting for D2 to finish the last one is sent once it has.
    pub(in crate::desktop::view::item) fn follow_registry(&mut self, cx: &mut Context<Self>) {
        let Some(run) = &self.edit.save.run else {
            return;
        };
        if run.direct.is_some() {
            return;
        }
        let (settling, request, waiting) = (run.settling, run.request, run.waiting());
        let live = self.live_draft(cx);
        if settling {
            if live.is_none() {
                self.end_run(cx);
            }
            return;
        }
        match request {
            // Waiting for Retry.
            Some(request) if self.edit.phase != Phase::Saving => {
                let held = live
                    .and_then(|id| self.services.drafts().read(cx).session(id))
                    .is_some_and(|session| {
                        matches!(
                            session.saving(),
                            Some(Saving::Submitted(saving, _)) if saving == request
                        )
                    });
                if !held {
                    self.edit.save.answer = None;
                    self.end_run(cx);
                }
            }
            Some(_) => {}
            None if waiting && self.edit.phase == Phase::Saving => self.send_save(cx),
            None => {}
        }
    }
}
