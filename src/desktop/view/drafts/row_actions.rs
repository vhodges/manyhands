//! The Drafts page's row actions: Restore, Open, Export…, Export raw… and
//! Discard…, the last three through the `actions` the tab shares.
//!
//! Restore leaves the page through its host, which opens the item's tab
//! (ruling R25); the page never hides itself. A comment draft's Open does
//! the same at once, claiming nothing: the item's composer adopts the
//! draft. What an action came to is
//! said on its row. A row Discard removes gives the focus to the next
//! row, else the previous one, else the page's first control.

use gpui_kit::{App, Context, Task, Window, component::WindowExt};
use manyhands::{
    canonical::ItemId,
    drafts::{DraftError, DraftId, DraftTarget, ItemKind, markdown},
    repository::ItemDtoKind,
};

use super::{
    ACTIONS, DraftsEvent, DraftsPage, Notice, Refocus, Slot,
    actions::{draft_name, export, may_open, open_discard, raw_name, suggested_name},
};
use crate::desktop::{
    model::{
        navigation::{ItemLocator, OpenTarget},
        tabs::TabKey,
    },
    services::drafts::RestoreRefused,
    view::item::{NOT_DISCARDED, locked_text, refused_label},
};

/// The desktop's item kind of a draft's.
fn dto_kind(kind: ItemKind) -> ItemDtoKind {
    match kind {
        ItemKind::Document => ItemDtoKind::Document,
        ItemKind::Ticket => ItemDtoKind::Ticket,
    }
}

/// The tab a draft of `target` is restored in, opened through its item
/// locator (ruling R25).
fn open_target(target: &DraftTarget) -> OpenTarget {
    let id: ItemId = target.item_id.clone();
    ItemLocator::Identified {
        kind: dto_kind(target.item_kind),
        id,
    }
    .in_repository(&target.repository)
}

/// The position of the action `part` in `ACTIONS`.
pub(super) fn action_ix(part: &str) -> usize {
    ACTIONS
        .iter()
        .position(|(name, _)| *name == part)
        .expect("a row action")
}

impl DraftsPage {
    /// Whether a tab of this window edits the item of the draft `id`, live
    /// or kept, or is getting a session ready: Restore waits, as a second
    /// session would be attached over the first (Part C fix 2, D; R-35).
    pub(super) fn edited_in_its_tab(&self, id: DraftId, cx: &App) -> bool {
        let Some(draft) = self.slot(id).and_then(|slot| slot.draft.as_ref()) else {
            return false;
        };
        let tab = TabKey::for_target(&open_target(&draft.target));
        self.services.edit_tabs().read(cx).editing(&tab, cx)
    }

    /// Restore: claims the draft and attaches it to its item's tab, which
    /// the host then opens and focuses. A refusal is said on the row. Not
    /// while a tab of this window edits the item.
    pub(super) fn restore(&mut self, id: DraftId, cx: &mut Context<Self>) {
        if self.edited_in_its_tab(id, cx) {
            return;
        }
        let Some(draft) = self.slot(id).and_then(|slot| slot.draft.as_ref()) else {
            return;
        };
        let target = open_target(&draft.target);
        // While the item's close or merge is unfinished, or its local copy
        // is being retired, its draft is not restored into its tab (D5 A2,
        // R-23; D6 decision 19).
        let kind = dto_kind(draft.target.item_kind);
        let lock = self.services.lock(
            &draft.target.repository,
            kind,
            &draft.target.item_id.to_string(),
            cx,
        );
        if draft.target.comment.is_none() && lock.blocks() {
            let words = locked_text(lock, kind);
            let notice = Notice {
                text: words.to_owned(),
                restore: false,
            };
            self.notices.insert(id, notice);
            cx.notify();
            return;
        }
        if !self.restoring.insert(id) {
            return;
        }
        let tab = TabKey::for_target(&target);
        self.notices.remove(&id);
        let drafts = self.services.drafts().clone();
        let restore = drafts.update(cx, |registry, cx| registry.restore(id, tab, cx));
        cx.spawn(async move |page, cx| {
            let restored = restore.await;
            let shown = page.update(cx, |page, cx| {
                page.restoring.remove(&id);
                match restored {
                    // Opened even when the user has gone on to Settings or
                    // a tab meanwhile: they asked for it, and the draft is
                    // claimed and attached now, so its tab is where it can
                    // be seen and closed (which lets it go again).
                    Ok(_) => cx.emit(DraftsEvent::Open(target)),
                    Err(refused) => {
                        let notice = Notice {
                            text: refused_label(refused),
                            restore: matches!(refused, RestoreRefused::OpenElsewhere),
                        };
                        page.notices.insert(id, notice);
                    }
                }
                cx.notify();
            });
            // The page went while the claim was taken: nothing opens the
            // tab, so the draft is let go of and offered again.
            if let (Err(_), Ok(restored)) = (shown, restored) {
                drafts.update(cx, |registry, cx| {
                    registry.release_when_stored(restored, cx)
                });
            }
        })
        .detach();
    }

    /// Open: the host opens and focuses the tab of a comment draft's item,
    /// whose composer adopts the draft. Nothing is claimed here.
    pub(super) fn open_item_of(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let Some(Slot {
            row,
            draft: Some(draft),
            ..
        }) = self.slot(id)
        else {
            return;
        };
        if !row.actions.open {
            return;
        }
        cx.emit(DraftsEvent::Open(open_target(&draft.target)));
    }

    /// Export… or Export raw…: a draft restored here exports its newest
    /// text; another readable one as listed; an unreadable one its bytes.
    pub(super) fn export(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        if !may_open(&self.services, window, cx) {
            return;
        }
        let drafts = self.services.drafts().clone();
        let (name, bytes) = match self.slot(id) {
            Some(Slot {
                draft: Some(draft), ..
            }) => {
                let draft = drafts
                    .read(cx)
                    .session(id)
                    .map(|session| session.draft())
                    .unwrap_or_else(|| draft.clone());
                let bytes = markdown(&draft).into_bytes();
                (suggested_name(&draft), Task::ready(Ok(bytes)))
            }
            Some(_) => (
                raw_name(id),
                drafts.update(cx, |registry, cx| registry.raw_bytes(id, cx)),
            ),
            None => return,
        };
        let export = export(name, bytes, cx);
        cx.spawn(async move |page, cx| {
            let exported = export.await;
            let _ = page.update(cx, |page, cx| {
                if let Some(text) = exported.label() {
                    let notice = Notice {
                        text,
                        restore: false,
                    };
                    page.notices.insert(id, notice);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Discard…: asks first, naming the draft as its row does. Not while a
    /// save of it is in flight.
    pub(super) fn discard(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving(id, cx) {
            return;
        }
        let (Some(slot), Some(focus)) = (self.slot(id), self.focus.get(&id)) else {
            return;
        };
        let readable = slot.draft.is_some();
        let name = draft_name(&slot.row);
        let origin = focus.actions[action_ix("discard")].clone();
        let page = cx.entity().downgrade();
        open_discard(name, origin, &self.services, window, cx, move |_, cx| {
            let _ = page.update(cx, |page, cx| page.discard_confirmed(id, readable, cx));
        });
    }

    /// Discards the draft `id`: through its session when restored here,
    /// else from the store. Once its row goes, the focus goes to a
    /// neighbour. A failure is said on the row; the draft is then
    /// released, still stored, and listed again.
    fn discard_confirmed(&mut self, id: DraftId, readable: bool, cx: &mut Context<Self>) {
        let drafts = self.services.drafts().clone();
        let held_here = drafts.read(cx).session(id).is_some();
        let discard = drafts.update(cx, |registry, cx| match held_here {
            true => registry.discard_held(id, cx),
            false => registry.discard_stored(id, readable, cx),
        });
        self.notices.remove(&id);
        let at = self.slots.iter().position(|slot| slot.row.id == id);
        let neighbour = |step: isize| {
            at.and_then(|at| at.checked_add_signed(step))
                .and_then(|ix| self.slots.get(ix))
                .map(|slot| slot.row.id)
        };
        let next = [neighbour(1), neighbour(-1)]
            .into_iter()
            .flatten()
            .collect();
        self.refocus = Some(Refocus { removed: id, next });
        cx.spawn(async move |page, cx| {
            if let Err(kind) = discard.await {
                let _ = page.update(cx, |page, cx| {
                    page.refocus = None;
                    let notice = Notice {
                        text: format!("{NOT_DISCARDED}: {}", DraftError { kind }),
                        restore: false,
                    };
                    page.notices.insert(id, notice);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Where the focus goes once a discarded row has gone: drawn each
    /// frame, it acts only then. (An Inspect panel that goes with its row
    /// needs nothing here: the workspace's focus-lost handler gives the
    /// focus to the page's first control.) Not while a dialog is open or
    /// quitting waits, with Quit now holding the focus: the refocus is kept
    /// for the first frame after, and goes to a neighbour still listed then,
    /// else the page's first control. Behind a sheet, Settings over the
    /// page, it is dropped, never kept for the sheet's close, which gives
    /// focus back where Settings opened from (Wave 04 Cycle 04, R-34).
    pub(super) fn apply_refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.refocus.is_none() || !may_open(&self.services, window, cx) {
            return;
        }
        let listed = |page: &Self, id: &DraftId| page.slots.iter().any(|slot| slot.row.id == *id);
        if let Some(refocus) = &self.refocus
            && !listed(self, &refocus.removed)
        {
            let next = refocus.next.iter().find(|id| listed(self, id)).copied();
            self.refocus = None;
            if window.has_active_sheet(cx) {
                return;
            }
            let focus = next
                .and_then(|id| self.focus.get(&id).map(|focus| focus.row.clone()))
                .or_else(|| self.first_focus());
            if let Some(focus) = focus {
                window.focus(&focus, cx);
            }
        }
    }
}
