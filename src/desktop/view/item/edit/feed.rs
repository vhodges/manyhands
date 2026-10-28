//! The session's D2 draft: begun at the first difference from the base,
//! fed every later change (R-12), attached to the tab, and read back for
//! the toolbar's status item, which keeps no state of its own.

use gpui_kit::Context;
use manyhands::drafts::{DraftErrorKind, DraftFields, DraftId, DraftTarget};

use super::{DraftLink, item_kind};
use crate::desktop::{
    model::{
        drafts::RecoveryState,
        edit::{DraftState, fields_with_body},
        navigation::OpenTarget,
    },
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
        let text = session.host.read(cx).text(cx);
        let current = fields_with_body(&session.base.fields, text);
        match session.draft {
            DraftLink::Live(id) => self
                .services
                .drafts()
                .update(cx, |registry, cx| registry.edit(id, current, cx)),
            DraftLink::Beginning => {}
            DraftLink::None | DraftLink::Failed => {
                let unavailable = self.services.drafts().read(cx).unavailable().is_some();
                if current != session.base.fields && !unavailable {
                    self.begin(current, cx);
                }
            }
        }
    }

    /// Begins the draft with `current` over the session's base, then
    /// attaches it to this tab.
    fn begin(&mut self, current: DraftFields, cx: &mut Context<Self>) {
        let OpenTarget::Item { repo, kind, id } = self.model.target().clone() else {
            return;
        };
        let Some(session) = &mut self.edit.session else {
            return;
        };
        session.draft = DraftLink::Beginning;
        let target = DraftTarget {
            repository: repo,
            item_kind: item_kind(kind),
            item_id: id,
            comment: None,
        };
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
    /// next difference begins again.
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
                let text = session.host.read(cx).text(cx);
                let now = fields_with_body(&session.base.fields, text);
                self.services.drafts().update(cx, |registry, cx| {
                    registry.attach(tab, id, cx);
                    if now != begun_with {
                        registry.edit(id, now, cx);
                    }
                });
            }
            Err(_) => session.draft = DraftLink::Failed,
        }
        self.sync_status(cx);
        cx.notify();
    }

    /// Gives the toolbar's status item D2's state of the session's text.
    pub(in crate::desktop::view::item) fn sync_status(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &self.edit.session else {
            return;
        };
        let registry = self.services.drafts().read(cx);
        let state = if registry.unavailable().is_some() {
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
