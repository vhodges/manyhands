//! The retirement loop at the scheduler (D6 design E1, RT-1 to RT-11,
//! R-17, R-32, R-36, R-37): after a completed attempt, one loop over the
//! items it reported, one background `ContextRetire` at a time.
//!
//! The rules are `model::polling::retire_loop`'s and pure. This module
//! reads the facts they need (the worker, the in-flight record, the refused
//! set, the open tabs, the draft registry), marks an item "Retiring…" in the
//! same UI update that chose it (RT-6), lists the draft store again off the
//! UI thread, checks again and submits in one turn, and ends the loop at
//! every exit: items run out, a wall (RT-8), a request that never ran
//! (RT-9), a reconfiguration (RT-10), a quit, a listing that failed, a
//! repository forgotten, or a submit that was refused. While a loop runs
//! its repository is in `Phase::Retiring` and no automatic poll starts
//! (R-32), so no exit may be left out.

use gpui_kit::{App, Context, WeakEntity};
use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, RequestId},
};

use super::{entity::Scheduler, reconfigure::scope_of};
use crate::desktop::{
    model::{
        operations::RetiringKey,
        polling::{
            AttemptTarget, PollSchedule, Root, TargetKind,
            entry::RetireOffer,
            retire::Verdict,
            retire_loop::{Candidate, Facts, Held, Next, Rechecked, Settled},
        },
        tabs::TabKey,
    },
    services::{
        drafts::{DraftRegistry, Fresh},
        edit_tabs::EditTabs,
        inflight::InflightStore,
        requests::{DesktopRequestId, Lane},
        worker::WorkerHandle,
    },
};

/// The item's kind as the tabs and the draft store know it.
pub(super) fn dto_kind(kind: TargetKind) -> ItemDtoKind {
    match kind {
        TargetKind::Ticket => ItemDtoKind::Ticket,
        TargetKind::Document => ItemDtoKind::Document,
    }
}

/// The key the "Retiring…" state is held under.
pub(super) fn key_of(root: &Root, kind: TargetKind, item: &ItemId) -> RetiringKey {
    (root.clone(), dto_kind(kind), item.to_string())
}

/// RT-1: whether a retirement of the item is queued or running in the worker
/// or held by the in-flight record, pending included.
pub(super) fn retirement_out(
    worker: &WorkerHandle,
    store: &InflightStore,
    root: &Root,
    kind: TargetKind,
    item: &ItemId,
) -> bool {
    let queued = retirement_out_in_worker(worker, root, kind, item);
    let entered = store.retirements().iter().any(|entry| {
        entry.root() == root
            && entry
                .item()
                .is_some_and(|(of, id)| of == kind && id == *item)
    });
    queued || entered
}

/// Whether a retirement of the item is queued, running or waiting out `busy`
/// in the worker (`WorkerHandle::retirement_in_flight`, which every such
/// question goes through).
pub(super) fn retirement_out_in_worker(
    worker: &WorkerHandle,
    root: &Root,
    kind: TargetKind,
    item: &ItemId,
) -> bool {
    worker.retirement_in_flight(root, dto_kind(kind), &item.to_string())
}

/// What the loop reads of one item. Borrowed field by field from the
/// scheduler, so the loop itself can be borrowed mutably beside it.
struct Probe<'a> {
    worker: &'a WorkerHandle,
    store: &'a InflightStore,
    schedule: &'a PollSchedule,
    tabs: &'a WeakEntity<EditTabs>,
    drafts: &'a WeakEntity<DraftRegistry>,
}

impl Probe<'_> {
    /// RT-1 to RT-5 for `candidate`, as they stand now. An open tab or a
    /// draft registry that cannot be read is not known to be free: it
    /// counts as busy, and as an unknown listing.
    fn facts(&self, root: &Root, candidate: &Candidate, cx: &App) -> Facts {
        let (kind, item) = (candidate.kind, &candidate.item);
        let in_flight = retirement_out(self.worker, self.store, root, kind, item);
        let tab = TabKey::Item {
            repo: root.clone(),
            kind: dto_kind(kind),
            id: item.to_string(),
        };
        Facts {
            retired: self.schedule.retire_line_retired(root, kind, item),
            in_flight,
            refused: self.schedule.retirement_refused(root, item),
            busy: self
                .tabs
                .read_with(cx, |tabs, cx| tabs.item_busy(&tab, cx))
                .unwrap_or(true),
            drafts: self
                .drafts
                .read_with(cx, |drafts, _| drafts.held_by(root, dto_kind(kind), item))
                .unwrap_or(Held::Unknown),
        }
    }
}

impl Scheduler {
    /// The items Retire… is offered for in `root` (E4): the schedule's, less
    /// the ones a loop that is running will still take up.
    pub(crate) fn retire_offers(&self, root: &Root) -> Vec<RetireOffer> {
        let coming = self
            .loops
            .get(root)
            .map(|lp| lp.remaining())
            .unwrap_or_default();
        self.schedule
            .retire_offers(root)
            .into_iter()
            .filter(|offer| {
                !coming
                    .iter()
                    .any(|next| next.kind == offer.kind && next.item == offer.item)
            })
            .collect()
    }

    /// A Check now completed (E1): attention is cleared first, then its
    /// attempt is the repository's latest, the index is refreshed if it
    /// was behind (R-10), and its items replace whatever the loop had left
    /// (RT-11).
    pub(super) fn checked_now(
        &mut self,
        root: Root,
        attempt: Option<crate::desktop::model::polling::PollAttempt>,
        cx: &mut Context<Self>,
    ) {
        self.checked(&root, cx);
        let Some(attempt) = attempt else {
            return;
        };
        let now = self.clock.now();
        let Some(summary) = self.schedule.checked_now(&now, &attempt) else {
            return;
        };
        if summary.index_pending {
            self.refresh_index(&root);
        }
        self.retire_after(&root, &summary.retire, cx);
        self.publish(cx);
    }

    /// A completed attempt reported `targets` as closed or promoted: they
    /// replace the repository's loop (RT-11) and a new one walks them. No
    /// loop starts while quitting is asked (R-33), for a second instance
    /// (L-6), while a reconfiguration of the repository is queued or running
    /// (RT-10), or when the repository cannot begin one (it waits for the
    /// person, or a poll of it is in flight).
    pub(super) fn retire_after(
        &mut self,
        root: &Root,
        targets: &[AttemptTarget],
        cx: &mut Context<Self>,
    ) {
        self.end_loop(root, cx);
        let items = Candidate::of(targets);
        if items.is_empty()
            || self.quitting
            || !self.store.runs_retirement_loop()
            || self.reconfiguring(root)
            || !self.schedule.begin_retiring(root)
        {
            return;
        }
        self.loops.begin(root, items);
        self.drive(root, cx);
    }

    /// Whether a request that reconfigures polling for `root` is queued or
    /// running (R-11).
    fn reconfiguring(&self, root: &Root) -> bool {
        self.worker.in_flight(|intent| {
            scope_of(intent, false).is_some_and(|scope| self.reached(&scope).contains(root))
        })
    }

    /// Takes the loop's next item that passes the first check, marks it
    /// "Retiring…" and lists the draft store again (RT-6); the loop ends
    /// when no item is left or quitting is asked.
    fn drive(&mut self, root: &Root, cx: &mut Context<Self>) {
        if self.quitting {
            return self.end_loop(root, cx);
        }
        let probe = Probe {
            worker: &self.worker,
            store: &self.store,
            schedule: &self.schedule,
            tabs: &self.edit_tabs,
            drafts: &self.drafts,
        };
        let Some(lp) = self.loops.get_mut(root) else {
            return;
        };
        let next = lp.next(|candidate| probe.facts(root, candidate, cx));
        let generation = lp.generation();
        match next {
            Next::Done => self.end_loop(root, cx),
            Next::Check(candidate) => {
                let key = key_of(root, candidate.kind, &candidate.item);
                let mark = self
                    .operations
                    .update(cx, |operations, cx| {
                        let mark = operations.mark_retiring(key);
                        cx.notify();
                        mark
                    })
                    .ok();
                self.marked.extend(mark.map(|mark| (root.clone(), mark)));
                self.relist(root.clone(), generation, cx);
            }
        }
    }

    /// Lists the draft store again, off the UI thread, and meets the item
    /// again when it lands (RT-6).
    fn relist(&mut self, root: Root, generation: u64, cx: &mut Context<Self>) {
        let listing = self
            .drafts
            .update(cx, |registry, cx| registry.relist(cx))
            .ok();
        cx.spawn(async move |this, cx| {
            let fresh = match listing {
                Some(listing) => listing.await,
                None => Fresh::Failed,
            };
            let _ = this.update(cx, |scheduler, cx| {
                scheduler.relisted(root, generation, fresh, cx)
            });
        })
        .detach();
    }

    /// The fresh listing is in (or failed): the item's rules are checked
    /// again on the UI thread in the turn it is submitted. A listing that
    /// failed ends the loop. One a newer listing overtook is not known to
    /// be fresh, so the item is skipped as unknown, not retired on a
    /// possibly old listing. A rule that holds the item back clears its mark
    /// and goes on to the next.
    pub(super) fn relisted(
        &mut self,
        root: Root,
        generation: u64,
        fresh: Fresh,
        cx: &mut Context<Self>,
    ) {
        // The loop went meanwhile (a quit, a reconfiguration, a newer
        // attempt): its mark went with it.
        if self.loops.get(&root).map(|lp| lp.generation()) != Some(generation) {
            return;
        }
        if fresh == Fresh::Failed {
            self.end_loop(&root, cx);
            return self.tick(cx);
        }
        let probe = Probe {
            worker: &self.worker,
            store: &self.store,
            schedule: &self.schedule,
            tabs: &self.edit_tabs,
            drafts: &self.drafts,
        };
        let Some(lp) = self.loops.get_mut(&root) else {
            return;
        };
        let Some(candidate) = lp.checking().cloned() else {
            return;
        };
        let mut facts = probe.facts(&root, &candidate, cx);
        if fresh == Fresh::Overtaken {
            facts.drafts = Held::Unknown;
        }
        match lp.recheck(facts) {
            Rechecked::Nothing => {}
            Rechecked::Skipped(_) => {
                self.unmark(&root, cx);
                self.drive(&root, cx);
            }
            Rechecked::Submit(candidate) => {
                let sent =
                    self.retire_new(&root, candidate.kind, &candidate.item, Lane::Background);
                match sent {
                    Some((request, id)) => {
                        if let Some(lp) = self.loops.get_mut(&root) {
                            lp.submitted(request);
                        }
                        self.sent.insert(id, root.clone());
                        // Handed to the worker: its queue, then its entry,
                        // hold the state from here.
                        self.unmark(&root, cx);
                    }
                    None => self.end_loop(&root, cx),
                }
                self.publish(cx);
            }
        }
        self.tick(cx);
    }

    /// Clears the mark of the item `root`'s loop was checking, if any.
    fn unmark(&mut self, root: &Root, cx: &mut Context<Self>) {
        if let Some(mark) = self.marked.remove(root) {
            let _ = self.operations.update(cx, |operations, cx| {
                if operations.unmark_retiring(mark) {
                    cx.notify();
                }
            });
        }
    }

    /// The loop of `root` is over, whatever the reason: its item is no
    /// longer marked, the repository is polled again (R-32), and what it
    /// still had to retire waits for the next completed attempt. A request
    /// it sent that is still out answers on its own and finds no loop.
    /// Idempotent. The caller ticks.
    pub(super) fn end_loop(&mut self, root: &Root, cx: &mut Context<Self>) {
        self.loops.end(root);
        self.unmark(root, cx);
        self.schedule.end_retiring(root);
    }

    /// The retirement `id`, request `request`, of `root` answered with
    /// `verdict`. `stopped`: the person pressed Stop on it, which ends the
    /// loop only when the request is the loop's own (RT-9). The loop goes on
    /// to its next item when the answer is the loop's own and lets it
    /// (RT-7); an answer that hits a wall stops it, the loop's own or not
    /// (RT-8, R-37); any other answer is none of its business.
    pub(super) fn loop_answered(
        &mut self,
        id: DesktopRequestId,
        root: &Root,
        request: RequestId,
        verdict: &Verdict,
        stopped: bool,
        cx: &mut Context<Self>,
    ) {
        self.sent.remove(&id);
        let Some(lp) = self.loops.get_mut(root) else {
            return;
        };
        // Stop ends the loop that sent the request, not another request's
        // (RT-9): the person's own Retire… of another item, stopped, is
        // none of the loop's business. A wall ends it either way (RT-8).
        let own = lp.running() == Some(request);
        match lp.answered(request, verdict.goes_on() && !(own && stopped)) {
            Settled::Next => self.drive(root, cx),
            Settled::Stop => self.end_loop(root, cx),
            Settled::Unrelated => {}
        }
    }

    /// The retirement `id`, request `request`, was removed from the queue or
    /// never sent (RT-9): the loop that was waiting on it stops.
    pub(super) fn loop_gone(
        &mut self,
        id: DesktopRequestId,
        request: RequestId,
        cx: &mut Context<Self>,
    ) {
        self.sent.remove(&id);
        if let Some(root) = self.loops.owner_of(request)
            && self.loops.get_mut(&root).is_some_and(|lp| lp.gone(request))
        {
            self.end_loop(&root, cx);
        }
    }

    /// A retirement request ended with no answer to read (the model has
    /// none): a loop waiting on it stops, as for a request that failed with
    /// no data (R-37).
    pub(super) fn loop_lost(&mut self, id: DesktopRequestId, cx: &mut Context<Self>) {
        if let Some(root) = self.sent.remove(&id) {
            self.end_loop(&root, cx);
        }
    }

    /// Every loop ends: quitting was asked (R-33).
    pub(super) fn end_loops(&mut self, cx: &mut Context<Self>) {
        for root in self.loops.roots() {
            self.end_loop(&root, cx);
        }
    }

    /// Whether every repository in `Phase::Retiring` has a loop and every
    /// loop's repository is in it: a repository left in the phase would
    /// never be polled again; the marks of the items being checked belong
    /// to loops, and the request model holds exactly those marks.
    #[cfg(test)]
    pub(crate) fn loops_consistent(&self, cx: &App) -> bool {
        use crate::desktop::model::polling::Phase;
        let retiring = |root: &Root| {
            self.schedule
                .repo(root)
                .is_some_and(|repo| repo.phase == Phase::Retiring)
        };
        let marks = self
            .operations
            .read_with(cx, |operations, _| operations.marks_held())
            .unwrap_or_default();
        self.known
            .iter()
            .all(|root| retiring(root) == self.loops.get(root).is_some())
            && self.loops.roots().iter().all(retiring)
            && self
                .marked
                .keys()
                .all(|root| self.loops.get(root).is_some())
            && marks == self.marked.len()
    }
}
