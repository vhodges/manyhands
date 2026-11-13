//! New-item drafts (design, Create (decision 3); R-20, R-20b, R-21, R-42,
//! R-46): a new item's draft written before its context, and its create.
//!
//! - **R-20.** `new_item` writes the new item's draft with `begin_new`,
//!   over a base with neither observation nor source that holds the path
//!   and fields, and only once the store holds it sends `context prepare`
//!   with the Create intent, on the user lane. A crash at any point leaves
//!   a restorable draft.
//! - **R-46.** A store that did not open fails `begin_new` at once, and
//!   nothing is sent or provisioned: `RECOVERY_NEEDED` says why.
//! - **Create** (`create`) is the Save of a not-created draft. As a save
//!   does, it writes the newest revision first, then sends it, one at a
//!   time (`save`): `DocumentCreate`, or `TicketCreate` with the draft's
//!   fields and no initials (R-21). `for_drafts` routes its answer here.
//! - **Completion** (R-20b) on `ok` or `already_applied`: the item is read
//!   again, and the base becomes the read when its fields are the ones
//!   sent, or otherwise, the read having failed included, the fields sent,
//!   unverified, with the source `""` (`retire`). The marker clears either
//!   way; the draft then retires, or its newer text goes on over that base,
//!   as after a save.
//! - **`occupied_path`.** The path is read with `show_path` in the item's
//!   own context, whose worktree its Create prepare's answer gave
//!   (`context_prepared`). When the file there is this draft's item, the
//!   draft becomes an ordinary unverified draft (R-42), which its tab
//!   reviews: an earlier create's answer was lost. Otherwise, as for
//!   `initials_required`, the draft is kept as it is, and its tab says why
//!   (R-21, R-22). A read that could not tell, busy or failed, is
//!   remembered (`path_unread`), and the tab names no other file there;
//!   nothing at the path in the item's own context (`path_not_found`, or
//!   `invalid_path` for a context the index does not hold yet) tells: the
//!   item's own file is not there.
//! - **A lost answer.** A create the library offers to take again under the
//!   same request keeps its record (`save`), so the same request sent again
//!   completes the draft once it has created the item, once.
//!
//! Nothing here holds text but the draft's session and the fields sent.

use std::path::PathBuf;

use gpui_kit::{Context, Task};
use manyhands::{
    drafts::{
        DraftBase, DraftErrorKind, DraftFields, DraftId, DraftTarget, ItemKind, is_new_item_base,
    },
    repository::{AuthoringKind, ContextPrepareIntent, ItemDto},
    results::ResultCode,
};

use super::{DraftRegistry, item_kind, save::Verb};
use crate::desktop::{
    model::drafts::create::unmarked,
    services::{
        ReadKind, ReadResult, Reader,
        created_contexts::ContextKey,
        requests::{DesktopRequestId, Intent, Lane},
    },
};

/// R-46: why a new item cannot be created while the draft store is not
/// open (pre-flight ruling, Task 22).
pub(crate) const RECOVERY_NEEDED: &str = "Crash recovery is off, so a new item cannot be created.";

/// A new item's draft, stored and claimed, and the `context prepare` sent
/// after it (R-20): what the new tab shows and follows. It holds no text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct NewItem {
    pub(crate) draft: DraftId,
    pub(crate) prepare: DesktopRequestId,
}

impl DraftRegistry {
    /// Creates a new item's draft of `target` at revision 1, claimed, and
    /// its session, as `begin` does. `base` has neither observation nor
    /// source (R-20), and holds the path and the fields. A store that did
    /// not open answers at once with why, having written nothing (R-46).
    pub(crate) fn begin_new(
        &mut self,
        target: DraftTarget,
        base: DraftBase,
        current: DraftFields,
        cx: &mut Context<Self>,
    ) -> Task<Result<DraftId, DraftErrorKind>> {
        debug_assert!(is_new_item_base(&base), "a new item's base");
        if let Some(unavailable) = self.unavailable {
            return Task::ready(Err(unavailable.0));
        }
        self.begin(target, Some(base), current, cx)
    }

    /// R-20, in its order: the new item's draft is written with
    /// `begin_new`, and only once the store holds it is `context prepare`
    /// with the Create intent sent, on the user lane. Answers both, or why
    /// the draft could not be written: then nothing is sent, and nothing
    /// is provisioned (R-46).
    pub(crate) fn new_item(
        &mut self,
        target: DraftTarget,
        base: DraftBase,
        current: DraftFields,
        cx: &mut Context<Self>,
    ) -> Task<Result<NewItem, DraftErrorKind>> {
        let prepare = prepare_create(&target);
        let begun = self.begin_new(target, base, current, cx);
        cx.spawn(async move |registry, cx| {
            let draft = begun.await?;
            // The registry lives as long as the services.
            let prepare = registry
                .update(cx, |registry, _| {
                    registry.worker.submit(prepare, Lane::User, None, None)
                })
                .map_err(|_| DraftErrorKind::Io)?;
            Ok(NewItem { draft, prepare })
        })
    }

    /// Creates the item of the session `id`, a new item's draft (R-20b):
    /// its newest revision is written first, then sent as `DocumentCreate`
    /// or `TicketCreate`, one at a time, as `save` sends a save. Answers
    /// the request, or `None`, having sent nothing, while a save of the
    /// session is in flight or its revision retires, and when its base is
    /// not a new item's.
    pub(crate) fn create(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Option<DesktopRequestId>> {
        self.flush_and_send(id, Verb::Create, cx)
    }

    /// Whether the session `id`'s last create met `occupied_path` and the
    /// read of the path could not tell, busy or failed, whether another file
    /// is there or this draft's own item (Part D fix 1, M5).
    pub(crate) fn path_unread(&self, id: DraftId) -> bool {
        self.path_unread.contains(&id)
    }

    /// A `context prepare` with the Create intent of `key`'s item answered
    /// `ok`, with the context's `worktree`: where a create's
    /// `occupied_path` is read.
    pub(crate) fn context_prepared(&mut self, key: ContextKey, worktree: PathBuf) {
        self.contexts.insert(key, worktree);
    }

    /// A create of the session `draft` was answered `occupied_path`: the
    /// path its base holds is read with `show_path` in the item's own
    /// context (the root's copy when this process made none ready). The
    /// session stays saving until the read lands (`occupied_read`).
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    pub(super) fn check_occupied(&mut self, draft: DraftId, cx: &mut Context<Self>) {
        let Some(live) = self.sessions.get_mut(&draft) else {
            return;
        };
        let Some(path) = live.session.base().map(|base| PathBuf::from(&base.path)) else {
            // A create needs a base: not reached. The session may save again.
            live.session.set_saving(None);
            cx.notify();
            return;
        };
        let target = live.session.target().clone();
        // The key the Create prepare's answer was kept under (`for_drafts`).
        let worktree = ContextKey::of(&prepare_create(&target))
            .and_then(|key| self.contexts.get(&key).cloned());
        let own_context = worktree.is_some();
        let root = PathBuf::from(&target.repository);
        let reader = Reader::new(self.backend.clone(), cx.background_executor().clone());
        let read = cx.background_executor().spawn(async move {
            reader
                .call(ReadKind::DraftItem, move |service| {
                    let repo = service.resolve_repository(&root)?;
                    service.show_path(&repo, worktree.as_deref(), &path)
                })
                .await
        });
        cx.spawn(async move |registry, cx| {
            let read = read.await;
            let _ = registry.update(cx, |registry, cx| {
                registry.occupied_read(draft, &target, &read, own_context, cx)
            });
        })
        .detach();
    }

    /// The read of an occupied path landed, in the item's own context when
    /// `own_context`. When it is this draft's item, the draft goes on as an
    /// ordinary unverified draft over the fields its base holds (R-42),
    /// written at once; its tab reviews it. Anything else keeps the draft
    /// as it is. A read that could not tell whether this draft's item is
    /// there is remembered (`path_unread`): busy, or failed, but for
    /// `path_not_found` or `invalid_path` in the item's own context, where
    /// that item would be. Either way the session may save again.
    fn occupied_read(
        &mut self,
        draft: DraftId,
        target: &DraftTarget,
        read: &ReadResult<ItemDto>,
        own_context: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.current_id(draft) else {
            return;
        };
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        let session = &mut live.session;
        session.set_saving(None);
        // In the item's own context, no file at the path, or no context
        // there that the index holds as active (a context made for a new
        // item holds none until its create), tells that this draft's item
        // is not there.
        let told = match read {
            ReadResult::Ok(_) => true,
            ReadResult::Failed(failure) => {
                own_context
                    && matches!(
                        failure.code,
                        ResultCode::PathNotFound | ResultCode::InvalidPath
                    )
            }
            ReadResult::Busy => false,
        };
        match told {
            true => self.path_unread.remove(&id),
            false => self.path_unread.insert(id),
        };
        let own = matches!(read, ReadResult::Ok(item) if holds(item, target));
        let marked = session
            .base()
            .filter(|base| is_new_item_base(base))
            .cloned();
        if let (true, Some(base)) = (own, marked) {
            session.rebase(unmarked(base), cx.background_executor().now());
            self.schedule(id, cx);
        }
        cx.notify();
    }
}

/// The `context prepare` with the Create intent of `target`'s item.
fn prepare_create(target: &DraftTarget) -> Intent {
    Intent::ContextPrepare {
        root: PathBuf::from(&target.repository),
        kind: match target.item_kind {
            ItemKind::Document => AuthoringKind::Document,
            ItemKind::Ticket => AuthoringKind::Ticket,
        },
        id: target.item_id.clone(),
        intent: ContextPrepareIntent::Create,
    }
}

/// Whether `item`, read at a draft's path, is the draft's own item.
fn holds(item: &ItemDto, target: &DraftTarget) -> bool {
    item_kind(item.kind) == target.item_kind
        && item
            .id
            .as_deref()
            .is_some_and(|id| id == target.item_id.to_string())
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "create_tests.rs"]
mod tests;
