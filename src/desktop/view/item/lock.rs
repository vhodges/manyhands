//! The lifecycle lock in an item's tab (D5 design, A2 "Read-only while
//! unfinished"; rulings R-21 to R-23). While the item's close or merge is
//! unfinished (`model::lifecycle_lock`), the tab is read-only and says
//! "Closing…" or "Merging…": Edit, Continue editing, Save, Share, Move…,
//! Repair… and a comment's Submit wait, while typing in a composer goes on.
//! While nothing is known yet, they wait with no words.
//!
//! A session shown when a run of the item locks it ends if it is clean and
//! is kept if it holds unsaved text (R-23). A merge whose tidy-up stopped
//! for a change the person can undo says "Merged, but not tidied up:
//! {reason}", with Open folder and Check again, which reads the tab and the
//! repository's rows again and sends the same request again (L-4, R-22).

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{Sizable, button::Button, h_flex},
    div,
};
#[cfg(all(test, feature = "desktop-test-support"))]
use manyhands::repository::ItemDto;
use manyhands::repository::ItemDtoKind;

use super::{ItemPane, banners::Banner, text};
use crate::desktop::{
    model::{
        lifecycle_lock::{LockSource, LockState, Untidied},
        tabs::TabKey,
    },
    view::lifecycle::cleanup_reason,
};

/// What a ticket's tab says while its close is unfinished.
pub(crate) const CLOSING: &str = "Closing\u{2026}";
/// What a document's tab says while its merge is unfinished.
pub(crate) const MERGING: &str = "Merging\u{2026}";
/// The note around a control that waits for the lock, carrying its words.
pub(crate) const LOCKED_NOTE: &str = "item-locked-note";
/// Share's note while it waits for the lock.
pub(crate) const SHARE_LOCKED: &str = "item-share-locked";
pub(crate) const CHECK_AGAIN: &str = "Check again";

/// The words of an item of `kind` while its lifecycle is unfinished.
pub(crate) fn lock_words(kind: ItemDtoKind) -> &'static str {
    match kind {
        ItemDtoKind::Ticket => CLOSING,
        ItemDtoKind::Document => MERGING,
    }
}

/// "Merged, but not tidied up: {reason}".
pub(crate) fn not_tidied(untidied: Untidied) -> String {
    format!(
        "Merged, but not tidied up: {}",
        cleanup_reason(Some(untidied.reason))
    )
}

impl ItemPane {
    /// Where the tab's item stands under the lock; `Open` for an Inspect
    /// tab, which has no lifecycle.
    pub(crate) fn lock_state(&self, cx: &App) -> LockState {
        match self.tab_key() {
            TabKey::Item { repo, id, .. } => self.services.lifecycle_lock(&repo, &id, cx),
            TabKey::Inspect { .. } => LockState::Open,
        }
    }

    /// Whether the tab's edits wait for the lock.
    pub(super) fn edits_wait(&self, cx: &App) -> bool {
        self.lock_state(cx).blocks()
    }

    /// The tab's item's kind, for its words.
    fn lock_kind(&self) -> ItemDtoKind {
        match self.tab_key() {
            TabKey::Item { kind, .. } => kind,
            TabKey::Inspect { .. } => ItemDtoKind::Document,
        }
    }

    /// What the tab says while a source locks it; none while nothing is
    /// known yet (R-21) or nothing locks it.
    pub(super) fn locked_words(&self, cx: &App) -> Option<&'static str> {
        matches!(self.lock_state(cx), LockState::Locked(_)).then(|| lock_words(self.lock_kind()))
    }

    /// What a refused action says: the item's words, whatever locks it.
    pub(super) fn refused_words(&self) -> &'static str {
        lock_words(self.lock_kind())
    }

    /// `control` inside the lock's note while a source locks the tab;
    /// as it is otherwise.
    pub(super) fn locked_note(
        &self,
        id: &'static str,
        control: AnyElement,
        cx: &App,
    ) -> AnyElement {
        match self.locked_words(cx) {
            Some(words) => div()
                .id(id)
                .aria_label(words)
                .test_support()
                .flex_none()
                .child(control)
                .into_any_element(),
            None => control,
        }
    }

    /// Whether a run of the item, or its record, locks it: not only the
    /// dialog, whose save first goes on.
    pub(super) fn run_locks(&self, cx: &App) -> bool {
        matches!(
            self.lock_state(cx),
            LockState::Locked(LockSource::Worker | LockSource::Record)
        )
    }

    /// The lock moved: a session shown while a run of the item, or its
    /// record, locks it ends if clean and is kept otherwise (R-23). The
    /// dialog's own lock waits for its save first.
    pub(super) fn lock_changed(&mut self, cx: &mut Context<Self>) {
        if self.run_locks(cx) {
            self.lock_session(cx);
        }
    }

    /// "Merged, but not tidied up: {reason}", with Open folder and Check
    /// again (L-4).
    pub(super) fn not_tidied_banner(&self, cx: &mut Context<Self>) -> Option<(Banner, AnyElement)> {
        let LockState::Untidied(untidied) = self.lock_state(cx) else {
            return None;
        };
        let folder = self
            .model
            .item()
            .rows()
            .map(|item| PathBuf::from(&item.context.worktree));
        let open = folder.map(|folder| {
            Button::new("item-not-tidied-open-folder")
                .label(crate::desktop::view::lifecycle::OPEN_FOLDER)
                .small()
                .on_click(move |_: &ClickEvent, _, cx: &mut App| cx.open_with_system(&folder))
        });
        let check = Button::new("item-not-tidied-check-again")
            .label(CHECK_AGAIN)
            .small()
            .on_click(cx.listener(move |pane, _: &ClickEvent, _, cx| {
                pane.check_again_untidied(untidied, cx);
            }));
        let banner = h_flex()
            .flex_wrap()
            .gap_2()
            .items_center()
            .child(text("item-not-tidied", not_tidied(untidied)))
            .children(open)
            .child(check)
            .into_any_element();
        Some((Banner::NotTidied, banner))
    }

    /// Check again: the tab and its repository's rows are read again, and
    /// the request that stopped is sent again under its own ID (R-22).
    fn check_again_untidied(&mut self, untidied: Untidied, cx: &mut Context<Self>) {
        let root = self.repo().to_owned();
        self.services.operations().update(cx, |operations, cx| {
            operations.mark_changed(root, true);
            cx.notify();
        });
        self.services.worker().retry(untidied.request, None);
        cx.notify();
    }

    /// The item as the tab last read it.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn shown_item(&self) -> Option<&ItemDto> {
        self.model.item().rows()
    }
}
