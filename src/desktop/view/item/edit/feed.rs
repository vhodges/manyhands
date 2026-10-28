//! The session's D2 draft: begun at the first difference from the base,
//! fed every later change (R-12), attached to the tab, and read back for
//! the toolbar's status item, which keeps no state of its own.
//!
//! When a save retires the draft, D2 ends its session (fact 79): the next
//! difference begins a draft again, on the base the save left (R-69).
//! While a save with no D2 session settles, that base is not known yet, so
//! the text waits in the editor and is fed once it is (`saved`). While D2
//! retires a revision or stores the text again, the tab is still saving
//! (`store_settling`).

use gpui_kit::{App, Context};
use manyhands::drafts::{DraftErrorKind, DraftFields, DraftId, DraftTarget};

use super::{DraftLink, Phase, item_kind};
use crate::desktop::{
    model::{drafts::RecoveryState, edit::DraftState, navigation::OpenTarget},
    view::item::ItemPane,
};

impl ItemPane {
    /// The editor's text changed: the first difference from the base
    /// begins the draft; every later change, back to the base included,
    /// updates it.
    pub(super) fn feed(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &self.edit.session else {
            return;
        };
        if session.draft == DraftLink::Beginning {
            return;
        }
        let current = session.current(cx);
        if let Some(id) = self.live_draft(cx) {
            self.services
                .drafts()
                .update(cx, |registry, cx| registry.edit(id, current, cx));
            return;
        }
        if self.edit.save.run.is_some() {
            return;
        }
        let unavailable = self.services.drafts().read(cx).unavailable().is_some();
        if current != session.base.fields && !unavailable {
            self.begin(current, cx);
        }
    }

    /// The live D2 session the tab's draft goes on as: `None` before one
    /// begins, and once D2 has ended it.
    pub(super) fn live_draft(&self, cx: &App) -> Option<DraftId> {
        let DraftLink::Live(id) = self.edit.session.as_ref()?.draft else {
            return None;
        };
        self.services.drafts().read(cx).current_id(id)
    }

    /// Whether D2 is still finishing a save of the tab's draft: retiring
    /// its revision, or storing newer text again as a new draft. That is
    /// part of saving: D2 sends no save meanwhile.
    pub(super) fn store_settling(&self, cx: &App) -> bool {
        self.live_draft(cx)
            .and_then(|id| self.services.drafts().read(cx).session(id))
            .is_some_and(|session| session.retiring())
    }

    /// The store's target of this tab's item.
    pub(super) fn draft_target(&self) -> Option<DraftTarget> {
        let OpenTarget::Item { repo, kind, id } = self.model.target().clone() else {
            return None;
        };
        Some(DraftTarget {
            repository: repo,
            item_kind: item_kind(kind),
            item_id: id,
            comment: None,
        })
    }

    /// Begins the draft with `current` over the session's base, then
    /// attaches it to this tab.
    fn begin(&mut self, current: DraftFields, cx: &mut Context<Self>) {
        let Some(target) = self.draft_target() else {
            return;
        };
        let Some(session) = &mut self.edit.session else {
            return;
        };
        session.draft = DraftLink::Beginning;
        let base = Some(session.base.clone());
        let begun_with = current.clone();
        let begun = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.begin(target, base, current, cx));
        let drafts = self.services.drafts().clone();
        cx.spawn(async move |pane, cx| {
            let begun = begun.await;
            let landed = pane.update(cx, |pane, cx| pane.draft_begun(begun, begun_with, cx));
            // The tab closed meanwhile: nothing shows the session, so it is
            // let go of, its draft kept for the Drafts page, as a restore
            // does.
            if let (Err(_), Ok(id)) = (landed, begun) {
                drafts.update(cx, |registry, cx| registry.release_when_stored(id, cx));
            }
        })
        .detach();
    }

    /// `begin` landed. The draft is attached to the tab, and text typed
    /// meanwhile is written. A failure keeps the text in the editor; the
    /// next difference begins again. A save asked for meanwhile is sent
    /// now, through the draft or, with none, as the fields are.
    fn draft_begun(
        &mut self,
        begun: Result<DraftId, DraftErrorKind>,
        begun_with: DraftFields,
        cx: &mut Context<Self>,
    ) {
        let tab = self.tab_key();
        let Some(session) = &mut self.edit.session else {
            return;
        };
        match begun {
            Ok(id) => {
                session.draft = DraftLink::Live(id);
                let now = session.current(cx);
                self.services.drafts().update(cx, |registry, cx| {
                    registry.attach(tab, id, cx);
                    if now != begun_with {
                        registry.edit(id, now, cx);
                    }
                });
            }
            Err(_) => session.draft = DraftLink::Failed,
        }
        let waiting = self.edit.phase == Phase::Saving
            && self.edit.save.run.as_ref().is_some_and(|run| run.waiting());
        if waiting {
            self.send_save(cx);
        }
        self.sync_status(cx);
        // Done waits for a draft still beginning (design, Done and leaving,
        // step 4).
        self.follow_done(cx);
        cx.notify();
    }

    /// The fields besides the body changed, from the details sheet's
    /// controls: they mark the session dirty exactly as typing does, and
    /// feed the draft (design, Named controls in the details sheet). The
    /// body is the editor's, whatever `fields` holds.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 20.
    pub(in crate::desktop::view::item) fn change_fields(
        &mut self,
        fields: DraftFields,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &mut self.edit.session else {
            return;
        };
        session.meta = fields;
        self.feed(cx);
        cx.notify();
    }

    /// Gives the toolbar's status item D2's state of the session's text.
    /// A kept session's toolbar is not drawn: it says nothing, so its age
    /// does not tick.
    pub(in crate::desktop::view::item) fn sync_status(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &self.edit.session else {
            return;
        };
        let registry = self.services.drafts().read(cx);
        let state = if session.kept {
            DraftState::NotBegun
        } else if registry.unavailable().is_some() {
            DraftState::StoreUnavailable
        } else {
            match session.draft {
                DraftLink::None | DraftLink::Beginning => DraftState::NotBegun,
                DraftLink::Failed => DraftState::Failed,
                DraftLink::Live(id) => match registry.session(id) {
                    Some(live) => match live.recovery_state() {
                        RecoveryState::Failed(_) => DraftState::Failed,
                        RecoveryState::Saved(_) | RecoveryState::Pending => {
                            DraftState::Written(live.flushed_at())
                        }
                    },
                    None => DraftState::NotBegun,
                },
            }
        };
        let toolbar = session.toolbar.clone();
        toolbar.update(cx, |toolbar, cx| toolbar.set_status(state, cx));
    }
}
