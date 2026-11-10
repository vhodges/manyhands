//! One repository's retirement loop (D6 design E1, RT-1 to RT-11, R-32):
//! pure, so the rules are tested without a worker, a draft store or a
//! thread.
//!
//! The loop holds the items a completed attempt reported as `Closed` or
//! `Promoted`, in report order, and walks them one at a time. For each it is
//! given [`Facts`] and says whether the item is skipped ([`Facts::skip`],
//! RT-1 to RT-5), is to be marked and checked again on a fresh draft listing
//! ([`RetireLoop::next`], then [`RetireLoop::recheck`], RT-6), or is
//! submitted ([`RetireLoop::submitted`]). It submits the next item only when
//! the previous one has answered ([`RetireLoop::answered`], RT-7, RT-8) and
//! stops when the request ended unsent ([`RetireLoop::gone`], RT-9).
//!
//! The loop knows requests by the library's request ID, and loops by a
//! generation: an answer that belongs to an earlier loop, or to a request
//! the loop did not send, neither continues the loop nor ends it, unless the
//! answer is one that stops every loop of the repository (RT-8).
//! Nothing here holds a title: an item is its ID.

use std::collections::{BTreeMap, VecDeque};

use manyhands::{canonical::ItemId, repository::RequestId};

use super::{AttemptTarget, Root, TargetKind};

/// An item the loop may retire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub kind: TargetKind,
    pub item: ItemId,
}

impl Candidate {
    /// The items an attempt reported, each once, in report order: a
    /// duplicate target is checked once. A target without an ID or a kind
    /// is no candidate (the attempt's `retire` list holds none).
    pub(crate) fn of(targets: &[AttemptTarget]) -> Vec<Self> {
        let mut candidates: Vec<Self> = Vec::new();
        for target in targets {
            let (Some(item), Some(kind)) = (&target.item_id, target.kind) else {
                continue;
            };
            let candidate = Self {
                kind,
                item: item.clone(),
            };
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        candidates
    }
}

/// What the draft store holds of an item (RT-4, RT-5, R-36).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Held {
    /// The store is unavailable, its listing has not landed or failed, or
    /// it holds an entry that cannot be read: an unknown listing counts as
    /// a draft.
    Unknown,
    No,
    /// A draft of the item or of a comment on it, a live session's
    /// included.
    Yes,
}

/// What the scheduler could read of one item when the loop reached it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Facts {
    /// A retirement of the item is queued, running or pending (RT-1).
    pub in_flight: bool,
    /// The item is in the session's refused set (RT-2, R-21).
    pub refused: bool,
    /// The item is being edited, or a comment is being written (RT-3).
    pub busy: bool,
    pub drafts: Held,
}

/// Why an item is not retired by the loop. The item gets Retire… (E4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Skip {
    InFlight,
    Refused,
    Busy,
    Draft,
    Unknown,
}

impl Facts {
    /// The first rule that holds the item back, in RT order.
    pub(crate) fn skip(&self) -> Option<Skip> {
        if self.in_flight {
            Some(Skip::InFlight)
        } else if self.refused {
            Some(Skip::Refused)
        } else if self.busy {
            Some(Skip::Busy)
        } else {
            match self.drafts {
                Held::Yes => Some(Skip::Draft),
                Held::Unknown => Some(Skip::Unknown),
                Held::No => None,
            }
        }
    }
}

/// Where the loop stands with its current item.
#[derive(Debug)]
enum Step {
    /// No item is in hand.
    Between,
    /// The item is marked and the draft store is listed again (RT-6).
    Checking(Candidate),
    /// The item's request was submitted and has not answered.
    Running(RequestId),
}

/// What [`RetireLoop::next`] found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Next {
    /// The item passed the first check: mark it and list the store again.
    Check(Candidate),
    /// No item is left.
    Done,
}

/// What [`RetireLoop::recheck`] found on the fresh listing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Rechecked {
    /// Still clear: submit it, then call [`RetireLoop::submitted`].
    Submit(Candidate),
    /// A rule now holds it back: clear its mark and go on to the next.
    Skipped(Skip),
    /// The loop has no item being checked.
    Nothing,
}

/// What an answer meant to the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settled {
    /// The loop's own item answered and the loop goes on.
    Next,
    /// The loop ends: its item answered with a wall (RT-8), or another
    /// retirement of the repository did.
    Stop,
    /// Not the loop's business.
    Unrelated,
}

/// One repository's loop.
#[derive(Debug)]
pub(crate) struct RetireLoop {
    generation: u64,
    queue: VecDeque<Candidate>,
    step: Step,
    /// What the loop did not retire, and why: the items that get Retire….
    skipped: Vec<(Candidate, Skip)>,
}

impl RetireLoop {
    fn new(generation: u64, items: Vec<Candidate>) -> Self {
        Self {
            generation,
            queue: items.into(),
            step: Step::Between,
            skipped: Vec::new(),
        }
    }

    /// Which loop this is: an asynchronous step that outlives its loop
    /// finds a different generation, or none.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// The item being checked on a fresh listing, if any.
    pub(crate) fn checking(&self) -> Option<&Candidate> {
        match &self.step {
            Step::Checking(candidate) => Some(candidate),
            _ => None,
        }
    }

    /// The request the loop is waiting on, if any.
    pub(crate) fn running(&self) -> Option<RequestId> {
        match &self.step {
            Step::Running(request) => Some(*request),
            _ => None,
        }
    }

    /// The items the loop left, with why, in the order it reached them.
    #[cfg_attr(not(test), allow(dead_code))] // Read by the tests.
    pub(crate) fn skipped(&self) -> &[(Candidate, Skip)] {
        &self.skipped
    }

    /// Takes the next item that passes the first check (RT-1 to RT-5),
    /// skipping the rest.
    pub(crate) fn next(&mut self, mut facts: impl FnMut(&Candidate) -> Facts) -> Next {
        while let Some(candidate) = self.queue.pop_front() {
            match facts(&candidate).skip() {
                Some(skip) => self.skipped.push((candidate, skip)),
                None => {
                    self.step = Step::Checking(candidate.clone());
                    return Next::Check(candidate);
                }
            }
        }
        self.step = Step::Between;
        Next::Done
    }

    /// The fresh listing is in: the item is checked again on `facts`
    /// (RT-6).
    pub(crate) fn recheck(&mut self, facts: Facts) -> Rechecked {
        let Step::Checking(candidate) = &self.step else {
            return Rechecked::Nothing;
        };
        let candidate = candidate.clone();
        match facts.skip() {
            Some(skip) => {
                self.skipped.push((candidate, skip));
                self.step = Step::Between;
                Rechecked::Skipped(skip)
            }
            None => Rechecked::Submit(candidate),
        }
    }

    /// The checked item was submitted under `request`.
    pub(crate) fn submitted(&mut self, request: RequestId) {
        self.step = Step::Running(request);
    }

    /// The request `request` answered, and `goes_on` says whether its
    /// answer lets a loop go on (RT-7, RT-8).
    pub(crate) fn answered(&mut self, request: RequestId, goes_on: bool) -> Settled {
        let mine = self.running() == Some(request);
        if mine {
            self.step = Step::Between;
        }
        match (mine, goes_on) {
            (true, true) => Settled::Next,
            (_, false) => Settled::Stop,
            (false, true) => Settled::Unrelated,
        }
    }

    /// The request `request` ended removed from the queue or never sent:
    /// whether that stops the loop (RT-9).
    pub(crate) fn gone(&mut self, request: RequestId) -> bool {
        let mine = self.running() == Some(request);
        if mine {
            self.step = Step::Between;
        }
        mine
    }
}

/// The loops of every repository: at most one each (R-32).
#[derive(Debug, Default)]
pub(crate) struct RetireLoops {
    next: u64,
    loops: BTreeMap<Root, RetireLoop>,
}

impl RetireLoops {
    /// A loop over `items` for `root`, replacing the one it had (RT-11).
    /// The replaced loop is given back, so its mark can be cleared.
    pub(crate) fn begin(
        &mut self,
        root: &Root,
        items: Vec<Candidate>,
    ) -> (&mut RetireLoop, Option<RetireLoop>) {
        self.next += 1;
        let fresh = RetireLoop::new(self.next, items);
        let old = self.loops.insert(root.clone(), fresh);
        (self.loops.get_mut(root).expect("just inserted"), old)
    }

    /// The repository whose loop waits on `request`, if any.
    pub(crate) fn owner_of(&self, request: RequestId) -> Option<Root> {
        self.loops
            .iter()
            .find(|(_, lp)| lp.running() == Some(request))
            .map(|(root, _)| root.clone())
    }

    pub(crate) fn get_mut(&mut self, root: &Root) -> Option<&mut RetireLoop> {
        self.loops.get_mut(root)
    }

    pub(crate) fn get(&self, root: &Root) -> Option<&RetireLoop> {
        self.loops.get(root)
    }

    /// The loop of `root` is over.
    pub(crate) fn end(&mut self, root: &Root) -> Option<RetireLoop> {
        self.loops.remove(root)
    }

    /// The repositories that have a loop.
    pub(crate) fn roots(&self) -> Vec<Root> {
        self.loops.keys().cloned().collect()
    }
}

#[cfg(test)]
#[path = "retire_loop_tests.rs"]
mod tests;
