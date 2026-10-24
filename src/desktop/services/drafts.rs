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

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    sync::Arc,
};

use futures::future::Shared;
use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{
        DraftBase, DraftError, DraftErrorKind, DraftFields, DraftId, DraftStore, DraftTarget,
        Entry, Held, ItemKind, Listing,
    },
    repository::ItemDtoKind,
    results::ResultCode,
};

mod flush;
mod restore;
mod retire;
mod save;

pub(crate) use restore::RestoreRefused;

use super::{Backend, ReadFailure, ReadKind, requests::DesktopRequestId, worker::WorkerHandle};
use crate::desktop::model::{
    drafts::{DraftSession, RecoveryState},
    list_state::ListState,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
    tabs::TabKey,
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

/// The drafts this session knows of, the sessions it has begun and their
/// saves in flight, and the tabs they are attached to.
pub(crate) struct DraftRegistry {
    backend: Arc<dyn Backend>,
    /// Where saves are sent.
    worker: WorkerHandle,
    /// Why the store did not open; `None` when it did.
    unavailable: Option<DraftStoreUnavailable>,
    /// The last listing of the store.
    listing: ListState<Vec<Listing>>,
    /// Why the last listing failed: the store's kind, whose fixed reason
    /// the Drafts page shows.
    listing_failure: Option<DraftErrorKind>,
    /// How many listings have landed: a view rebuilds what it shows of
    /// the listing when this moves.
    listed: u64,
    /// The drafts the first listing to land held: those this launch
    /// recovered.
    launch: Option<BTreeSet<DraftId>>,
    /// The "Recovered drafts" entry was dismissed: for this session.
    recovered_dismissed: bool,
    generations: Generations,
    /// Replacing a listing drops the previous one.
    listing_read: Option<Task<()>>,
    sessions: BTreeMap<DraftId, Live>,
    /// Saves sent and not yet applied, by request.
    saves: BTreeMap<DesktopRequestId, save::SaveRecord>,
    /// Draft IDs whose session goes on under another: a session whose
    /// text was stored again as a new draft. A caller holding the old ID
    /// reaches the session through `current_id`.
    moved: BTreeMap<DraftId, DraftId>,
    /// The session each item tab holds, by the ID it was restored under;
    /// `attached` follows it to the ID it goes on under.
    attached: HashMap<TabKey, DraftId>,
    /// Sessions whose tab is gone and whose release waits for their text
    /// to be stored, by the ID they were released under.
    releasing: BTreeSet<DraftId>,
}

/// A session and the tasks that write it.
struct Live {
    session: DraftSession,
    /// Waits for the next due time; replacing it drops the earlier wait.
    timer: Option<Task<()>>,
    /// The last store call started on the claim (a write, a retirement or
    /// a store again), for a flush, a save or a discard to wait on.
    write: Option<Shared<Task<()>>>,
}

impl DraftRegistry {
    pub(crate) fn new(backend: Arc<dyn Backend>, worker: WorkerHandle) -> Self {
        Self {
            unavailable: backend.drafts().err(),
            backend,
            worker,
            listing: ListState::Loading,
            listing_failure: None,
            listed: 0,
            launch: None,
            recovered_dismissed: false,
            generations: Generations::default(),
            listing_read: None,
            sessions: BTreeMap::new(),
            saves: BTreeMap::new(),
            moved: BTreeMap::new(),
            attached: HashMap::new(),
            releasing: BTreeSet::new(),
        }
    }

    /// Why the store did not open, if it did not.
    pub(crate) fn unavailable(&self) -> Option<DraftStoreUnavailable> {
        self.unavailable
    }

    /// Lists the store again; an earlier listing is dropped when it
    /// completes. The listing already shown stays until the new one
    /// lands. A store that did not open is not listed.
    pub(crate) fn reload_listing(&mut self, cx: &mut Context<Self>) -> Stamp {
        let stamp = self.generations.next(ViewScope::Drafts);
        if self.unavailable.is_some() {
            self.listing_read = None;
            return stamp;
        }
        let list = off_thread(&self.backend, ReadKind::Drafts, DraftStore::list, cx);
        let asked = stamp.clone();
        self.listing_read = Some(cx.spawn(async move |registry, cx| {
            let result = list.await;
            // The registry lives as long as the services.
            let _ = registry.update(cx, |registry, cx| {
                if registry.apply_listing(asked, result) {
                    cx.notify();
                }
            });
        }));
        stamp
    }

    /// A listing completed. False when it is no longer wanted. A store
    /// that failed to list reads as a failed list, and keeps its kind for
    /// `listing_failure`; its error's text is not kept.
    pub(crate) fn apply_listing(
        &mut self,
        stamp: Stamp,
        result: Result<Vec<Listing>, DraftError>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        self.listed += 1;
        self.listing_failure = result.as_ref().err().map(|error| error.kind);
        self.listing = match result {
            Ok(rows) => {
                self.launch
                    .get_or_insert_with(|| rows.iter().map(|listing| listing.id).collect());
                ListState::Ready {
                    rows,
                    complete: true,
                    index: None,
                }
            }
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
    pub(crate) fn listing(&self) -> &ListState<Vec<Listing>> {
        &self.listing
    }

    /// Why the last listing failed, as the store's kind; `None` when it
    /// did not.
    pub(crate) fn listing_failure(&self) -> Option<DraftErrorKind> {
        self.listing_failure
    }

    /// How many listings have landed.
    pub(crate) fn listings_landed(&self) -> u64 {
        self.listed
    }

    /// How many of the drafts the launch found are still stored and not
    /// claimed by this process: the operation area's "Recovered drafts".
    /// `None` until the launch listing lands, when there are none, and
    /// once the entry is dismissed.
    pub(crate) fn recovered(&self) -> Option<usize> {
        if self.recovered_dismissed {
            return None;
        }
        let launch = self.launch.as_ref()?;
        let count = self
            .listing
            .rows()?
            .iter()
            .filter(|listing| listing.held != Held::Here && launch.contains(&listing.id))
            .count();
        (count > 0).then_some(count)
    }

    /// Dismisses the "Recovered drafts" entry until the next launch.
    pub(crate) fn dismiss_recovered(&mut self, cx: &mut Context<Self>) {
        self.recovered_dismissed = true;
        cx.notify();
    }

    /// The newest stored draft of the item `id` of `kind` in `repo` that
    /// this process has not claimed and can read. `None` while the store
    /// is unavailable or not yet listed. A comment's draft is not the
    /// item's.
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
    ///
    /// A caller that drops the task after the create ran but before the
    /// session is inserted releases the claim: revision 1 stays on disk as
    /// an unclaimed draft. No text is lost; the next listing offers it for
    /// recovery.
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
        let Some(id) = self.current_id(id) else {
            return;
        };
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        let before = live.session.recovery_state();
        live.session.edit(current, now);
        // Views show the state, not the text: a keystroke that leaves the
        // state as it was redraws nothing.
        let changed = live.session.recovery_state() != before;
        self.schedule(id, cx);
        if changed {
            cx.notify();
        }
    }

    /// The ID the session of `id` goes on under: `id` itself, or the draft
    /// its text was stored again as (and so on). `None` when no session
    /// of it is live. A tab holding `id` re-keys to the answer.
    pub(crate) fn current_id(&self, mut id: DraftId) -> Option<DraftId> {
        loop {
            if self.sessions.contains_key(&id) {
                return Some(id);
            }
            id = *self.moved.get(&id)?;
        }
    }

    /// Whether the newest text of `id` is safe in the store; `None` when
    /// this process has no session of it.
    pub(crate) fn recovery_state(&self, id: DraftId) -> Option<RecoveryState> {
        self.sessions
            .get(&self.current_id(id)?)
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

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_tests.rs"]
mod tests;
