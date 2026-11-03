//! The item header's Share (design, Lane B, Share controls): after Edit
//! or Save/Done, a secondary `cloud-upload` button that sends `item sync`
//! of the item. Its tooltip names the publication remote and says what is
//! sent; there is no Share dialog (decision 9). It is enabled whenever the
//! repository has a publication remote and no share of this item is
//! queued or running (decision 10): the library answers for an item with
//! no editing context. With no publication remote it reads Share…, which
//! opens Settings › Remotes for the repository (`ItemRoute::Remotes`).
//! While a share of the item is queued or running it reads Sharing… and is
//! disabled, so a second press queues nothing.

use std::path::{Path, PathBuf};

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{Disableable, Icon, Sizable, button::Button},
    div,
};
use manyhands::repository::{AuthoringKind, ItemDtoKind};

use super::{ItemPane, ItemRoute};
use crate::desktop::{
    assets::icon_path,
    model::{
        item::SHARE,
        navigation::OpenTarget,
        stamp::{Generations, SettingsPage, ViewScope},
    },
    services::{
        ReadKind, ReadResult,
        requests::{Intent, Lane, RequestState},
    },
    view::sidebar::SHARING,
};

/// Share's element: the Kit button, which takes the focus.
pub(crate) const SHARE_BUTTON: &str = "item-share";
/// Share's tooltip, as an element that carries it as its label.
pub(crate) const SHARE_NOTE: &str = "item-share-note";
/// Share with no publication remote: it opens Settings › Remotes.
pub(crate) const SHARE_SET_UP: &str = "Share…";

/// Share's tooltip, naming the publication remote `remote` (design, Share
/// controls).
pub(crate) fn share_tooltip(remote: &str) -> String {
    format!(
        "Brings in others' changes and shares this item's saved work through {remote}. \
         Unsaved edits are not included."
    )
}

/// The tab's Share: what it last read of the repository's publication
/// remote.
#[derive(Default)]
pub(super) struct ShareState {
    /// The publication remote's name, once read; `None` before the first
    /// read lands or when no remote is selected for publication.
    remote: Option<String>,
    /// Whether a read of the remotes has landed: until then Share is not
    /// drawn, so it never reads Share… for a repository that has a remote.
    known: bool,
    /// The last remotes read; a newer one drops it.
    read: Option<gpui_kit::Task<()>>,
    generations: Generations,
    /// How many remote requests of the repository had finished when the
    /// remotes were last read: one more finishing reads them again.
    remote_changes: usize,
}

/// Whether `intent` changes which remote the repository at `root`
/// publishes to.
fn changes_remotes(intent: &Intent, root: &Path) -> bool {
    match intent {
        Intent::RemoteAdd { root: asked, .. }
        | Intent::RemoteRemove { root: asked, .. }
        | Intent::RemoteSelect { root: asked, .. } => asked == root,
        _ => false,
    }
}

impl ItemPane {
    /// The item this tab shares: its repository, kind and ID, for an item
    /// tab. A file read by its path has nothing to share.
    fn share_target(&self) -> Option<Intent> {
        let OpenTarget::Item { repo, kind, id } = self.model.target() else {
            return None;
        };
        let kind = match kind {
            ItemDtoKind::Document => AuthoringKind::Document,
            ItemDtoKind::Ticket => AuthoringKind::Ticket,
        };
        Some(Intent::ItemSync {
            root: PathBuf::from(repo),
            kind,
            id: id.clone(),
        })
    }

    /// Whether a share of this item is queued, waiting out `busy`, or
    /// running: the worker's own account, so a press made before the
    /// request model has heard of the last one still finds it.
    fn sharing(&self) -> bool {
        let Some(Intent::ItemSync { root, kind, id }) = self.share_target() else {
            return false;
        };
        self.services.worker().in_flight(|intent| {
            matches!(
                intent,
                Intent::ItemSync { root: asked, kind: asked_kind, id: asked_id }
                    if *asked == root && *asked_kind == kind && *asked_id == id
            )
        })
    }

    /// The tab opened: the repository's publication remote is read into
    /// `ShareState`, so the header reads it from there.
    pub(super) fn share_opened(&mut self, cx: &mut Context<Self>) {
        if self.share_target().is_some() {
            self.read_share_remote(cx);
        }
    }

    /// Reads which remote, if any, the repository publishes to.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn read_share_remote(&mut self, cx: &mut Context<Self>) {
        let root = PathBuf::from(self.repo());
        // The read is the one the Remotes page makes, of the same rows.
        let stamp = self
            .share
            .generations
            .next(ViewScope::Settings(SettingsPage::Remotes));
        self.share.read = Some(self.services.read(
            ReadKind::Remotes,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.list_remotes_redacted(&repo)
            },
            cx,
            |pane: &mut Self, stamp, read, cx| {
                if !pane.share.generations.is_current(&stamp) {
                    return;
                }
                pane.share.read = None;
                // A failed read keeps what was known; Share stays as it was.
                if let ReadResult::Ok(list) = read {
                    pane.share.remote = list
                        .items
                        .into_iter()
                        .find(|remote| remote.selected_for_publication)
                        .map(|remote| remote.name);
                    pane.share.known = true;
                    cx.notify();
                }
            },
        ));
    }

    /// The request model changed: a remote of the repository added,
    /// removed or selected reads the publication remote again. Whether a
    /// share is running is the worker's, read as the header is drawn.
    pub(super) fn share_progressed(&mut self, cx: &mut Context<Self>) {
        if self.share_target().is_none() {
            return;
        }
        let root = PathBuf::from(self.repo());
        let finished = self
            .services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| {
                changes_remotes(&request.intent, &root)
                    && matches!(request.state, RequestState::Finished(..))
            })
            .count();
        if finished != self.share.remote_changes {
            self.share.remote_changes = finished;
            self.read_share_remote(cx);
        }
    }

    /// Share, after Edit or Save/Done in the header: for an item tab whose
    /// item has been read, once the remotes are. With a remote, its
    /// tooltip names it, carried by `SHARE_NOTE`'s label.
    pub(super) fn share_button(
        &self,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.share_target()?;
        if !self.share.known || self.model.item().rows().is_none() {
            return None;
        }
        let sharing = self.sharing();
        let label = match (&self.share.remote, sharing) {
            (_, true) => SHARING,
            (Some(_), false) => SHARE,
            (None, false) => SHARE_SET_UP,
        };
        let button = Button::new(SHARE_BUTTON)
            .label(label)
            .small()
            .icon(Icon::default().path(icon_path("cloud-upload")))
            .disabled(sharing)
            .on_click(cx.listener(|pane, _: &ClickEvent, window, cx| {
                pane.share_item(window, cx);
            }));
        let Some(remote) = &self.share.remote else {
            return Some(button.into_any_element());
        };
        let tooltip = share_tooltip(remote);
        Some(
            div()
                .id(SHARE_NOTE)
                .test_support()
                .aria_label(tooltip.clone())
                .flex_none()
                .child(button.tooltip(tooltip))
                .into_any_element(),
        )
    }

    /// `ShareItem`, or Share pressed: with a publication remote, sends
    /// `item sync` of the item unless a share of it is already queued or
    /// running; with none, asks the workspace for Settings › Remotes. True
    /// when Share is offered, so the action goes no further, even when it
    /// queued nothing.
    pub(super) fn share_item(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(intent) = self.share_target() else {
            return false;
        };
        if !self.share.known || self.model.item().rows().is_none() {
            return false;
        }
        if self.share.remote.is_none() {
            cx.emit(ItemRoute::Remotes(PathBuf::from(self.repo())));
            return true;
        }
        if !self.sharing() {
            self.services
                .worker()
                .submit(intent, Lane::User, None, None);
            cx.notify();
        }
        true
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "share_tests.rs"]
mod tests;
