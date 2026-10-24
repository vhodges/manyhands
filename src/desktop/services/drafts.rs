//! The draft registry: what this session knows of the draft store, shared
//! by the workspace, the tabs and the Drafts page.
//!
//! Store calls are blocking file I/O. They run on the background executor
//! as reads do, each behind the seam's gate of its `ReadKind`, and come
//! back to the asking entity with the stamp they were asked under, so a
//! late listing never replaces a newer one.
//!
//! When the store did not open, the desktop still starts: the registry
//! keeps why, lists nothing and offers no draft.
//!
//! The registry also holds the sessions this process has begun. Each
//! session says when its newest revision is due; the registry waits for
//! that on a timer of the background executor, whose clock tests advance,
//! and writes the revision off the foreground behind the `DraftWrite`
//! gate. The result lands back on the session, which schedules whatever
//! came meanwhile.

use std::{collections::BTreeMap, fmt, sync::Arc};

use futures::future::{FutureExt, Shared, join_all};
use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{
        DraftBase, DraftError, DraftErrorKind, DraftFields, DraftId, DraftStore, DraftTarget,
        Entry, Held, ItemKind, Listing, Written,
    },
    repository::ItemDtoKind,
    results::ResultCode,
};

use super::{Backend, ReadFailure, ReadKind, Services};
use crate::desktop::model::{
    drafts::{DraftSession, RecoveryState},
    list_state::ListState,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};

/// Why the draft store did not open: the desktop's copy of the error's
/// kind. Its text is the store's fixed sentence for that kind; it carries
/// no path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct DraftStoreUnavailable(pub DraftErrorKind);

impl From<DraftError> for DraftStoreUnavailable {
    fn from(error: DraftError) -> Self {
        Self(error.kind)
    }
}

impl fmt::Display for DraftStoreUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        DraftError { kind: self.0 }.fmt(formatter)
    }
}

/// The store's item kind of a desktop item.
fn item_kind(kind: ItemDtoKind) -> ItemKind {
    match kind {
        ItemDtoKind::Document => ItemKind::Document,
        ItemDtoKind::Ticket => ItemKind::Ticket,
    }
}

/// The drafts this session knows of, and the sessions it has begun. Later
/// it also holds the tabs they are attached to, and the saves in flight.
pub(crate) struct DraftRegistry {
    backend: Arc<dyn Backend>,
    /// Why the store did not open; `None` when it did.
    unavailable: Option<DraftStoreUnavailable>,
    /// The last listing of the store.
    listing: ListState<Vec<Listing>>,
    generations: Generations,
    /// Replacing a listing drops the previous one.
    listing_read: Option<Task<()>>,
    sessions: BTreeMap<DraftId, Live>,
}

/// A session and the tasks that write it.
struct Live {
    session: DraftSession,
    /// Waits for the next due time; replacing it drops the earlier wait.
    timer: Option<Task<()>>,
    /// The last write started, for a flush to wait on.
    write: Option<Shared<Task<()>>>,
}

/// What a flush does next.
enum Flush {
    Done(Result<(), DraftErrorKind>),
    /// Wait for this write; `started` when this flush started it.
    Wait {
        write: Shared<Task<()>>,
        started: bool,
    },
}

impl DraftRegistry {
    pub(crate) fn new(backend: Arc<dyn Backend>) -> Self {
        Self {
            unavailable: backend.drafts().err(),
            backend,
            listing: ListState::Loading,
            generations: Generations::default(),
            listing_read: None,
            sessions: BTreeMap::new(),
        }
    }

    /// Why the store did not open, if it did not.
    pub(crate) fn unavailable(&self) -> Option<DraftStoreUnavailable> {
        self.unavailable
    }

    /// Lists the store again; an earlier listing is dropped when it
    /// completes. The listing already shown stays until the new one
    /// lands. A store that did not open is not listed.
    pub(crate) fn reload_listing(&mut self, services: &Services, cx: &mut Context<Self>) -> Stamp {
        let stamp = self.generations.next(ViewScope::Drafts);
        if self.unavailable.is_some() {
            self.listing_read = None;
            return stamp;
        }
        self.listing_read = Some(services.draft_read(
            ReadKind::Drafts,
            stamp.clone(),
            DraftStore::list,
            cx,
            |registry: &mut Self, stamp, result, cx| {
                if registry.apply_listing(stamp, result) {
                    cx.notify();
                }
            },
        ));
        stamp
    }

    /// A listing completed. False when it is no longer wanted. A store
    /// that failed to list reads as a failed list; its error's text is
    /// not kept.
    pub(crate) fn apply_listing(
        &mut self,
        stamp: Stamp,
        result: Result<Vec<Listing>, DraftError>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        self.listing = match result {
            Ok(rows) => ListState::Ready {
                rows,
                complete: true,
                index: None,
            },
            // The store's kinds are not result codes: the list reads as
            // failed, as a read the library could not finish does.
            Err(_) => ListState::Failed(ReadFailure {
                code: ResultCode::InternalError,
                recovery: Vec::new(),
            }),
        };
        true
    }

    /// The last listing of the store.
    #[cfg_attr(not(test), allow(dead_code))] // Read by the Drafts page; tests until then.
    pub(crate) fn listing(&self) -> &ListState<Vec<Listing>> {
        &self.listing
    }

    /// The newest stored draft of the item `id` of `kind` in `repo` that
    /// this process has not claimed and can read. `None` while the store
    /// is unavailable or not yet listed. A comment's draft is not the
    /// item's.
    #[cfg_attr(not(test), allow(dead_code))] // Asked by a tab's banner; tests until then.
    pub(crate) fn stored_for(
        &self,
        repo: &RepoKey,
        kind: ItemDtoKind,
        id: &str,
    ) -> Option<DraftId> {
        if self.unavailable.is_some() {
            return None;
        }
        let kind = item_kind(kind);
        // An ID the item type does not take names no stored draft.
        let id: ItemId = id.parse().ok()?;
        self.listing.rows()?.iter().find_map(|listing| {
            let Entry::Draft(draft) = &listing.entry else {
                return None;
            };
            let target = &draft.target;
            (listing.held != Held::Here
                && target.comment.is_none()
                && target.item_kind == kind
                && target.repository == *repo
                && target.item_id == id)
                .then_some(listing.id)
        })
    }

    /// Creates a draft of `target` at revision 1, claimed, and its
    /// session: D3's first edit. On failure nothing is stored, and the
    /// caller keeps the text and may begin again.
    #[cfg_attr(not(test), allow(dead_code))] // Called by D3's editor; tests until then.
    pub(crate) fn begin(
        &mut self,
        target: DraftTarget,
        base: Option<DraftBase>,
        current: DraftFields,
        cx: &mut Context<Self>,
    ) -> Task<Result<DraftId, DraftErrorKind>> {
        // A create takes the draft's claim, so it waits behind `Drafts`.
        let create = off_thread(
            &self.backend,
            ReadKind::Drafts,
            move |store| store.create(target, base, current),
            cx,
        );
        cx.spawn(async move |registry, cx| {
            let claim = create.await.map_err(|error| error.kind)?;
            registry
                .update(cx, |registry, cx| {
                    let session = DraftSession::new(claim, cx.background_executor().now());
                    let id = session.id();
                    let live = Live {
                        session,
                        timer: None,
                        write: None,
                    };
                    registry.sessions.insert(id, live);
                    cx.notify();
                    id
                })
                // The registry lives as long as the services; gone, the
                // claim is released with the draft stored.
                .map_err(|_| DraftErrorKind::Io)
        })
    }

    /// The editor's newest text of `id`, scheduled to be written.
    #[cfg_attr(not(test), allow(dead_code))] // Called by D3's editor; tests until then.
    pub(crate) fn edit(&mut self, id: DraftId, current: DraftFields, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        live.session.edit(current, now);
        self.schedule(id, cx);
        cx.notify();
    }

    /// Whether the newest text of `id` is safe in the store; `None` when
    /// this process has no session of it.
    #[cfg_attr(not(test), allow(dead_code))] // Shown by Task 8's tab; tests until then.
    pub(crate) fn recovery_state(&self, id: DraftId) -> Option<RecoveryState> {
        self.sessions
            .get(&id)
            .map(|live| live.session.recovery_state())
    }

    /// Each session whose last write failed: its ID, its item's title as
    /// read and why.
    pub(crate) fn flush_failures(&self) -> Vec<(DraftId, String, DraftErrorKind)> {
        self.sessions
            .values()
            .filter_map(|live| match live.session.recovery_state() {
                RecoveryState::Failed(kind) => Some((
                    live.session.id(),
                    live.session.item_title().to_owned(),
                    kind,
                )),
                _ => None,
            })
            .collect()
    }

    /// Writes the newest revision of `id` now, after any write in flight:
    /// Retry, a close and a quit. Answers once it is written, or why the
    /// write this flush made failed.
    pub(crate) fn flush_now(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        cx.spawn(async move |registry, cx| {
            let mut attempted = false;
            loop {
                let step = registry
                    .update(cx, |registry, cx| registry.flush_step(id, attempted, cx))
                    .map_err(|_| DraftErrorKind::NotFound)?;
                match step {
                    Flush::Done(result) => return result,
                    Flush::Wait { write, started } => {
                        attempted |= started;
                        write.await;
                    }
                }
            }
        })
    }

    /// Flushes every session now; answers the ones that failed.
    #[cfg_attr(not(test), allow(dead_code))] // Called by Task 9's quit; tests until then.
    pub(crate) fn flush_all(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Task<Vec<(DraftId, DraftErrorKind)>> {
        let ids: Vec<DraftId> = self.sessions.keys().copied().collect();
        let flushes: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let flush = self.flush_now(id, cx);
                async move { (id, flush.await) }
            })
            .collect();
        cx.spawn(async move |_, _| {
            join_all(flushes)
                .await
                .into_iter()
                .filter_map(|(id, result)| result.err().map(|kind| (id, kind)))
                .collect()
        })
    }

    fn flush_step(&mut self, id: DraftId, attempted: bool, cx: &mut Context<Self>) -> Flush {
        let Some(live) = self.sessions.get_mut(&id) else {
            return Flush::Done(Err(DraftErrorKind::NotFound));
        };
        if live.session.writing()
            && let Some(write) = live.write.clone()
        {
            return Flush::Wait {
                write,
                started: false,
            };
        }
        match live.session.recovery_state() {
            RecoveryState::Saved(_) => Flush::Done(Ok(())),
            RecoveryState::Failed(kind) if attempted => Flush::Done(Err(kind)),
            _ => {
                live.timer = None;
                match self.start_write(id, cx) {
                    Some(write) => Flush::Wait {
                        write,
                        started: true,
                    },
                    None => Flush::Done(Ok(())),
                }
            }
        }
    }

    /// Waits on a timer for the session's due time, then writes it.
    fn schedule(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        let Some(due) = live.session.flush_due() else {
            live.timer = None;
            return;
        };
        let wait = due.saturating_duration_since(now);
        live.timer = Some(cx.spawn(async move |registry, cx| {
            cx.background_executor().timer(wait).await;
            let _ = registry.update(cx, |registry, cx| registry.start_write(id, cx));
        }));
    }

    /// Takes the session's newest revision and writes it off the
    /// foreground. `None` when nothing is new or a write is in flight.
    fn start_write(&mut self, id: DraftId, cx: &mut Context<Self>) -> Option<Shared<Task<()>>> {
        let live = self.sessions.get_mut(&id)?;
        let write = live.session.next_write()?;
        let work = off_thread(
            &self.backend,
            ReadKind::DraftWrite,
            move |_| write.write(),
            cx,
        );
        let task = cx
            .spawn(async move |registry, cx| {
                let result = work.await;
                let _ = registry.update(cx, |registry, cx| registry.write_landed(id, result, cx));
            })
            .shared();
        live.write = Some(task.clone());
        Some(task)
    }

    fn write_landed(
        &mut self,
        id: DraftId,
        result: Result<Written, DraftError>,
        cx: &mut Context<Self>,
    ) {
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        live.session.write_landed(result);
        self.schedule(id, cx);
        cx.notify();
    }
}

/// Runs `f` against the store on the background executor after the gate
/// of `kind`, taken now, in the order calls are made. A store that did not
/// open answers with why.
fn off_thread<V: 'static, T: Send + 'static>(
    backend: &Arc<dyn Backend>,
    kind: ReadKind,
    f: impl FnOnce(&DraftStore) -> Result<T, DraftError> + Send + 'static,
    cx: &mut Context<V>,
) -> Task<Result<T, DraftError>> {
    let backend = backend.clone();
    let gate = backend.read_gate(kind);
    cx.background_executor().spawn(async move {
        if let Some(gate) = gate {
            // A dropped sender releases the call as a send does.
            let _ = gate.await;
        }
        let store = backend.drafts().map_err(|unavailable| DraftError {
            kind: unavailable.0,
        })?;
        f(store)
    })
}

impl Services {
    /// Runs `f` against the draft store on the background executor, after
    /// the gate of `kind`, and hands its result to `apply` on the asking
    /// entity, with `stamp`. A store that did not open answers with why.
    /// `apply` is not called if the entity is gone.
    pub(crate) fn draft_read<V: 'static, T: Send + 'static>(
        &self,
        kind: ReadKind,
        stamp: Stamp,
        f: impl FnOnce(&DraftStore) -> Result<T, DraftError> + Send + 'static,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, Result<T, DraftError>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let read = off_thread(&self.backend, kind, f, cx);
        cx.spawn(async move |entity, cx| {
            let result = read.await;
            // The entity may be gone; then nobody wants the result.
            let _ = entity.update(cx, |entity, cx| apply(entity, stamp, result, cx));
        })
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_tests.rs"]
mod tests;
