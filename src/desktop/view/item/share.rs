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
//!
//! While a share or a post of the item runs, the pill says its stage
//! (`share_progress`, design, Stage text), from the request model. Once
//! the item's newest share has finished, and until its operation area
//! entry is dismissed, the pill says its outcome while no edit session
//! says more, the item reads as its shared or its private copy (or the
//! share stopped on a conflict), and the item still reads as it did when
//! it was first read after the share finished (`read_after_share`):
//! sharing private work changes what the item reads, so the read before
//! the share never speaks for its outcome. A conflict raises the item's
//! conflict banner, which offers nothing (`share_outcome`, decision 13).
//!
//! The remote is the workspace's navigation's (`publication_remote`,
//! decision 20): the tab reads no remotes itself (D3's fact 41). Until
//! navigation has listed the repository, Share is not drawn.

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Task, TestSupportExt, Window,
    component::{Disableable, Icon, Sizable, button::Button},
    div,
};
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, ItemDto, ItemDtoKind},
};

use super::{ItemPane, ItemRoute, PublicationRemote};
use crate::desktop::{
    assets::icon_path,
    model::{
        item::SHARE,
        navigation::OpenTarget,
        operations::{Progressing, progress_words},
        publication::SyncOutcome,
        stamp::{Generations, TabId, ViewScope},
    },
    services::{
        ReadKind, ReadResult, Reader,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    view::{
        operations::{share_result, share_words},
        sidebar::SHARING,
    },
};

/// The scope of the read after a share (`read_after_share`). The tab's
/// share state keeps its own generations, so it is no tab's but its own.
const AFTER_SHARE: ViewScope = ViewScope::Tab(TabId(0));

/// The item `id` of the repository at `root`'s observation as it reads
/// now; `None` when it cannot be read.
#[allow(clippy::result_large_err)]
async fn observation_of(reader: Reader, root: PathBuf, id: ItemId) -> Option<Option<String>> {
    let repo = match reader
        .call(ReadKind::Resolve, |service| {
            service.resolve_repository(&root)
        })
        .await
    {
        ReadResult::Ok(repo) => repo,
        _ => return None,
    };
    match reader
        .call(ReadKind::Item, |service| service.show_item(&repo, &id))
        .await
    {
        ReadResult::Ok(item) => Some(item.observation),
        _ => None,
    }
}

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

/// The tab's Share: what its pill last said of a running share or post.
/// The remote is the pane's (`publication_remote`), and whether a share
/// runs is the worker's.
#[derive(Default)]
pub(super) struct ShareState {
    progress: Option<String>,
    pub(super) outcome: Option<ShareOutcome>,
    /// The share whose outcome the pill may say, and the item's
    /// `observation` as the item was first read after the share finished,
    /// once that read has landed: a later read that differs ends it in the
    /// pill.
    seen: Option<(DesktopRequestId, Option<Option<String>>)>,
    /// That read, while it runs, and its generations: a read for an older
    /// share is dropped.
    after: Option<Task<()>>,
    generations: Generations,
}

/// What the item's newest finished share, while its operation area entry
/// is listed, says in the pill, and whether it stopped on a conflict.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ShareOutcome {
    pub(super) request: DesktopRequestId,
    pub(super) words: String,
    pub(super) conflict: bool,
    /// Every outcome but shared, already current and nothing to share
    /// needs attention.
    pub(super) attention: bool,
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

    /// The tab opened. Nothing to read: the workspace gives the remote
    /// (`share_remote_changed`) once its navigation lists the repository.
    pub(super) fn share_opened(&mut self, _cx: &mut Context<Self>) {}

    /// The request model changed: a share or post of the item may have
    /// reached another stage, so the pill is drawn again. Whether a share
    /// runs is read from the worker as the header is drawn, and a finished
    /// remote request reloads navigation, which gives the remote again.
    pub(super) fn share_progressed(&mut self, cx: &mut Context<Self>) {
        let progress = self.share_progress(cx);
        let outcome = self.share_outcome(cx);
        // The item as it is first read after the share finished is what
        // the outcome speaks of: read here when the outcome is first seen,
        // as the share's own reload of the tab reads it.
        if let Some(outcome) = &outcome
            && self.share.seen.as_ref().map(|(id, _)| *id) != Some(outcome.request)
        {
            self.share.seen = Some((outcome.request, None));
            self.read_after_share(outcome.request, cx);
        }
        if self.share.progress != progress || self.share.outcome != outcome {
            self.share.progress = progress;
            self.share.outcome = outcome;
            cx.notify();
        }
    }

    /// What the item's newest share (`item sync`) says once finished,
    /// until its operation area entry is dismissed (design, Outcomes and
    /// routes): the pill's words, and whether the item's conflict banner
    /// shows. A failure says nothing here; its entry shows the library's
    /// message.
    pub(super) fn share_outcome(&self, cx: &App) -> Option<ShareOutcome> {
        let Some(Intent::ItemSync { root, kind, id }) = self.share_target() else {
            return None;
        };
        let latest = self
            .services
            .operations()
            .read(cx)
            .latest_item_share(&root, kind, &id)?;
        let result = share_result(&self.services, latest, cx)?;
        Some(ShareOutcome {
            request: latest,
            words: share_words(&self.services, latest, &result, cx)?,
            conflict: matches!(result.outcome, SyncOutcome::Conflict { .. }),
            attention: !matches!(
                result.outcome,
                SyncOutcome::Published | SyncOutcome::Current | SyncOutcome::NothingToShare
            ),
        })
    }

    /// Reads the item once the share `request` has finished, for the
    /// observation its outcome speaks of. A read that fails leaves the
    /// outcome out of the pill; its entry still says it.
    fn read_after_share(&mut self, request: DesktopRequestId, cx: &mut Context<Self>) {
        let OpenTarget::Item { repo, id, .. } = self.model.target() else {
            return;
        };
        let (root, id) = (PathBuf::from(repo), id.clone());
        let stamp = self.share.generations.next(AFTER_SHARE);
        self.share.after = Some(self.services.read_job(
            stamp,
            move |reader| observation_of(reader, root, id),
            cx,
            move |pane: &mut Self, stamp, observation, cx| {
                // A read for an older share leaves the newer one running.
                if !pane.share.generations.is_current(&stamp) {
                    return;
                }
                pane.share.after = None;
                if let (Some((seen, after)), Some(observation)) =
                    (&mut pane.share.seen, observation)
                    && *seen == request
                {
                    *after = Some(observation);
                    cx.notify();
                }
            },
        ));
    }

    /// The outcome the pill may say: while `read`, the item as the tab
    /// read it, reads as the item was first read after the share finished.
    pub(super) fn share_outcome_shown(&self, read: Option<&ItemDto>) -> Option<&ShareOutcome> {
        let outcome = self.share.outcome.as_ref()?;
        let (id, Some(observation)) = self.share.seen.as_ref()? else {
            return None;
        };
        (*id == outcome.request && read.map(|item| &item.observation) == Some(observation))
            .then_some(outcome)
    }

    /// What the pill says while a share (`item sync`) or a post
    /// (`comment add`) of this item runs, newest first: "Sharing · …" or
    /// "Posting · …", naming the publication remote while connecting.
    pub(super) fn share_progress(&self, cx: &App) -> Option<String> {
        let OpenTarget::Item { repo, kind, id } = self.model.target() else {
            return None;
        };
        let root = PathBuf::from(repo);
        let kind = match kind {
            ItemDtoKind::Document => AuthoringKind::Document,
            ItemDtoKind::Ticket => AuthoringKind::Ticket,
        };
        let operations = self.services.operations().read(cx);
        let (progressing, stage) = operations.requests().iter().find_map(|request| {
            let RequestState::Running { stage, .. } = request.state else {
                return None;
            };
            let ours = match &request.intent {
                Intent::ItemSync {
                    root: asked,
                    kind: asked_kind,
                    id: asked_id,
                }
                | Intent::CommentAdd {
                    root: asked,
                    kind: asked_kind,
                    id: asked_id,
                    ..
                } => *asked == root && *asked_kind == kind && asked_id == id,
                _ => false,
            };
            ours.then(|| Progressing::of(&request.intent).map(|kind| (kind, stage)))
                .flatten()
        })?;
        let remote = match self.publication_remote() {
            PublicationRemote::Named(name) => Some(name.as_str()),
            PublicationRemote::Unknown | PublicationRemote::Unset => None,
        };
        Some(progress_words(progressing, stage, remote))
    }

    /// The repository's publication remote, as navigation gives it,
    /// changed: Share is drawn again with it.
    pub(super) fn share_remote_changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Share, after Edit or Save/Done in the header: for an item tab whose
    /// item has been read, once navigation has given the remote. With a
    /// remote, its tooltip names it, carried by `SHARE_NOTE`'s label.
    pub(super) fn share_button(
        &self,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.share_target()?;
        self.model.item().rows()?;
        let remote = match self.publication_remote() {
            PublicationRemote::Unknown => return None,
            PublicationRemote::Unset => None,
            PublicationRemote::Named(name) => Some(name),
        };
        let sharing = self.sharing();
        let label = match (remote, sharing) {
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
        let Some(remote) = remote else {
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
        if self.model.item().rows().is_none() {
            return false;
        }
        match self.publication_remote() {
            PublicationRemote::Unknown => return false,
            PublicationRemote::Unset => {
                cx.emit(ItemRoute::Remotes(PathBuf::from(self.repo())));
                return true;
            }
            PublicationRemote::Named(_) => {}
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
