//! Release on leave (design, Done and leaving: Release on leave; decision
//! 18; R-37, R-40, R-75, R-78): `context release` of an item's private work
//! context is sent when all four of the design's conditions hold.
//!
//! 1. The context is in `CreatedContexts`: this process made it, and no
//!    save has committed into it since. Any other context is never
//!    released by leaving or by Discard. Nor is one this service has
//!    released already, until a prepare of this process makes it again: a
//!    context made again by another process after the release is theirs
//!    (decision 18), though the set still holds the key.
//! 2. The item's tab leaves clean (`leave`: Done, or closing), or a draft
//!    of the item was discarded, from a tab, its banners or the Drafts
//!    page, with or without a tab (the registry's `DraftDiscardLanded`).
//! 3. No save of the item is in flight: queued or running on the worker,
//!    or held by a session of this process.
//! 4. The draft store answers, and lists no draft of the item from any
//!    process. The store is listed again once the leave, and so its own
//!    discard, has landed, and the release waits for that listing. A store
//!    that did not open, or a listing that failed, releases nothing. A
//!    comment's draft on the item counts; one that cannot be read names no
//!    item.
//!
//! Nor is a context released while a tab of this process edits its item,
//! live or kept, Preparing and Loading included (controller ruling, Task
//! 17 fix 1): each item tab says so (`watch`), and the release happens when
//! that tab leaves.
//!
//! A tab closed while its Edit prepares leaves the prepare here: once it
//! answers `ok` with `created: true`, release on leave applies as for a
//! clean tab; any other answer releases nothing (R-37, Known Limits).
//!
//! R-78: a release is never sent across a later prepare of its item. A
//! prepare of the item sent since the leave began (for a closed tab's
//! prepare, since that prepare), or still queued or running once the store
//! has answered, stops the release; and the worker never sends a context
//! request again once a later one of its item would undo it
//! (`Intent::undone_by`).
//!
//! The release goes on the user lane, after what the user asked for before
//! it, and counts as released as it is sent: a refusal is shown in the
//! operation area, and the context stays (Known Limits).
//! Quitting releases nothing. "Release stranded work context" (R-40) sends
//! a release at the user's word, whoever made the context; the library
//! refuses one that holds work.
//!
//! Nothing here holds text: a key names an item by its root, kind and ID.

use std::collections::{HashMap, HashSet};

use gpui_kit::{App, Context, Entity, Subscription, WeakEntity};
use manyhands::{
    drafts::{DraftTarget, Entry, ItemKind},
    repository::ItemDtoKind,
};

use super::{
    created_contexts::{ContextKey, CreatedContexts},
    drafts::{DraftDiscardLanded, DraftRegistry},
    requests::{DesktopRequestId, Intent, Lane},
    worker::{SendMark, WorkerHandle},
};

/// A release waiting for the draft store's listing (condition 4).
struct Check {
    /// How many listings had landed when the store was listed again: the
    /// next to land is this leave's, or a later one.
    landed: u64,
    /// The newest attempt when the leave began, or the closed prepare's own
    /// (R-78).
    since: SendMark,
}

/// The releases this process sends on leave, and what each waits for.
pub(crate) struct Releases {
    worker: WorkerHandle,
    created: Entity<CreatedContexts>,
    drafts: WeakEntity<DraftRegistry>,
    /// Releases waiting for the store's listing, by item.
    checks: HashMap<ContextKey, Check>,
    /// Prepares whose tab closed while they ran, by request (R-37).
    closed: HashMap<DesktopRequestId, ContextKey>,
    /// The contexts whose release was sent, until a prepare of this
    /// process makes one again.
    released: HashSet<ContextKey>,
    /// What each item tab says of its item (`watch`).
    editing: Vec<(ContextKey, Editing)>,
    _subscriptions: [Subscription; 2],
}

/// What an item tab says of its item: `Some(true)` while it holds an edit
/// session of it, live or kept, from Preparing through Loading; `None` once
/// the tab is gone.
pub(crate) type Editing = Box<dyn Fn(&App) -> Option<bool>>;

impl Releases {
    pub(crate) fn new(
        worker: WorkerHandle,
        created: Entity<CreatedContexts>,
        drafts: &Entity<DraftRegistry>,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = [
            cx.observe(drafts, |releases, drafts, cx| releases.listed(&drafts, cx)),
            cx.subscribe(drafts, |releases, _, event: &DraftDiscardLanded, cx| {
                releases.discarded(&event.0, cx)
            }),
        ];
        Self {
            worker,
            created,
            drafts: drafts.downgrade(),
            checks: HashMap::new(),
            closed: HashMap::new(),
            released: HashSet::new(),
            editing: Vec::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Condition 1: whether `key`'s context is one this process made, saved
    /// nothing into and has not released since.
    pub(crate) fn made_here(&self, key: &ContextKey, cx: &App) -> bool {
        self.created.read(cx).contains(key) && !self.released.contains(key)
    }

    /// A tab of `key`'s item says, through `editing`, whether it edits the
    /// item: no release of it is decided while one does.
    pub(crate) fn watch(&mut self, key: ContextKey, editing: Editing) {
        self.editing.push((key, editing));
    }

    /// Condition 2: the tab of `key`'s item left clean, or a draft of the
    /// item was discarded. Only a context this process made goes on to the
    /// store's listing; one already waiting for it is not listed twice.
    pub(crate) fn leave(&mut self, key: ContextKey, cx: &mut Context<Self>) {
        let since = self.worker.newest();
        self.leave_since(key, since, cx);
    }

    /// `leave`, with a prepare of the item sent after `since` stopping the
    /// release (R-78).
    fn leave_since(&mut self, key: ContextKey, since: SendMark, cx: &mut Context<Self>) {
        if self.checks.contains_key(&key) || self.worker.quitting() || !self.made_here(&key, cx) {
            return;
        }
        let Some(drafts) = self.drafts.upgrade() else {
            return;
        };
        // A store that did not open is never listed: nothing is released.
        if drafts.read(cx).unavailable().is_some() {
            return;
        }
        let landed = drafts.update(cx, |registry, cx| {
            registry.reload_listing(cx);
            registry.listings_landed()
        });
        self.checks.insert(key, Check { landed, since });
    }

    /// R-37: the tab of `key`'s item closed while its prepare `request`
    /// ran. Release on leave applies once it answers `ok` with `created:
    /// true`.
    pub(crate) fn closed_while_preparing(&mut self, request: DesktopRequestId, key: ContextKey) {
        self.closed.insert(request, key);
    }

    /// The prepare `request` of `item` finished, or was removed or not
    /// sent; `made` when it answered `ok` with `created: true`. A context
    /// made again counts as unreleased.
    pub(crate) fn prepared(
        &mut self,
        request: DesktopRequestId,
        item: Option<ContextKey>,
        made: bool,
        cx: &mut Context<Self>,
    ) {
        if made && let Some(item) = &item {
            self.released.remove(item);
        }
        let Some(key) = self.closed.remove(&request) else {
            return;
        };
        if made {
            // The mark is the prepare's own: one sent after it, by a tab
            // that opened the item again, stops the release (M1).
            let since = self
                .worker
                .sent_mark(request)
                .unwrap_or_else(|| self.worker.newest());
            self.leave_since(key, since, cx);
        }
    }

    /// R-40: "Release stranded work context", at the user's word. It is
    /// sent whoever made the context.
    pub(crate) fn stranded(&mut self, key: ContextKey) {
        self.send(key);
    }

    /// Whether a release of `key`'s context is queued or running, or waits
    /// for the store's listing or for a prepare's answer: Move is disabled
    /// meanwhile.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 24.
    pub(crate) fn pending(&self, key: &ContextKey) -> bool {
        self.checks.contains_key(key)
            || self.closed.values().any(|closed| closed == key)
            || self.worker.in_flight(|intent| key.released_by(intent))
    }

    /// A draft was discarded: a release of its item is considered, as for
    /// a clean leave. A comment's draft is not the item's.
    fn discarded(&mut self, target: &DraftTarget, cx: &mut Context<Self>) {
        if target.comment.is_none() {
            self.leave(key_of(target), cx);
        }
    }

    /// The registry changed: each release whose listing has landed since
    /// is decided on it.
    fn listed(&mut self, drafts: &Entity<DraftRegistry>, cx: &mut Context<Self>) {
        if self.checks.is_empty() {
            return;
        }
        let decided: Vec<(ContextKey, Check, bool)> = {
            let registry = drafts.read(cx);
            let landed = registry.listings_landed();
            let due: Vec<ContextKey> = self
                .checks
                .iter()
                .filter(|(_, check)| landed > check.landed)
                .map(|(key, _)| key.clone())
                .collect();
            due.into_iter()
                .map(|key| {
                    let check = self.checks.remove(&key).expect("listed above");
                    let free = lists_none(registry, &key)
                        && !registry.saving_item(|target| names(target, &key));
                    (key, check, free)
                })
                .collect()
        };
        for (key, check, free) in decided {
            if free {
                self.decide(key, check.since, cx);
            }
        }
    }

    /// Once the store has answered: condition 1 again (a save may have
    /// committed meanwhile), condition 3 on the worker, R-78, and no tab of
    /// this process editing the item.
    fn decide(&mut self, key: ContextKey, since: SendMark, cx: &mut Context<Self>) {
        let prepared = |intent: &Intent| key.prepared_by(intent);
        if self.worker.quitting()
            || !self.made_here(&key, cx)
            || self.worker.in_flight(|intent| key.saved_by(intent))
            || self.worker.sent_since(since, prepared)
            || self.worker.in_flight(prepared)
            || self.edited(&key, cx)
        {
            return;
        }
        self.send(key);
    }

    /// Whether a tab of this process edits `key`'s item, live or kept,
    /// Preparing and Loading included (controller ruling, Task 17 fix 1).
    /// The release happens when that tab leaves. A tab that is gone is
    /// forgotten. It is read only here, as the registry's listing lands,
    /// when no tab is being updated.
    fn edited(&mut self, key: &ContextKey, cx: &App) -> bool {
        self.editing.retain(|(_, editing)| editing(cx).is_some());
        self.editing
            .iter()
            .any(|(item, editing)| item == key && editing(cx) == Some(true))
    }

    /// Sends the release of `key`'s context on the user lane; it counts as
    /// released.
    fn send(&mut self, key: ContextKey) {
        let Some(intent) = key.release() else {
            return;
        };
        self.worker.submit(intent, Lane::User, None, None);
        self.released.insert(key);
    }
}

/// The context key of a draft's item.
fn key_of(target: &DraftTarget) -> ContextKey {
    let kind = match target.item_kind {
        ItemKind::Document => ItemDtoKind::Document,
        ItemKind::Ticket => ItemDtoKind::Ticket,
    };
    ContextKey::new(&target.repository, kind, &target.item_id.to_string())
}

/// Whether a draft's target names `key`'s item, a comment on it included.
fn names(target: &DraftTarget, key: &ContextKey) -> bool {
    key_of(target) == *key
}

/// Condition 4: the last listing is ready and holds no draft of `key`'s
/// item, from any process.
fn lists_none(registry: &DraftRegistry, key: &ContextKey) -> bool {
    registry.listing().rows().is_some_and(|rows| {
        !rows.iter().any(|listing| match &listing.entry {
            Entry::Draft(draft) => names(&draft.target, key),
            Entry::Unreadable(_) => false,
        })
    })
}
