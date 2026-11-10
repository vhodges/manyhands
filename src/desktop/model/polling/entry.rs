//! The attempt entry's content (D6 design A4, R-20, R-23, W-6): pure, from
//! what an attempt summed to.
//!
//! Each repository has at most one entry in the operation area, the latest
//! attempt's, replaced by the next. It says what an attempt brought in and
//! lists each item the poll left alone, with its text. It is shown only when
//! it has something to say: a quiet "up to date" attempt (`Current`,
//! `LocalAhead`) shows no entry and updates the Settings row alone.
//!
//! The text of a disposition is the library's `guidance()`, except where it
//! names a CLI command: the desktop names its own action instead (R-20).
//! `Unpublished` says so and offers Share…. `Closed` and `Promoted` are not
//! listed from the attempt alone: the items the retirement loop left alone
//! join the entry as offers of Retire… (`AttemptEntry::with_offers`, E4),
//! from the schedule's account of what was reported, retired and refused.
//! The retirement lines of the entry (E3) follow the requests; the failure
//! and the attention slots are Task 8's.
//!
//! Nothing here names a title, a body or a comment: only an item's ID and
//! the library's fixed words.

use manyhands::{canonical::ItemId, repository::PollDisposition};

use super::{AttemptSummary, Disposition, TargetKind, status::changes};

/// Share…, the action `Unpublished` offers (R-20).
pub(crate) const SHARE: &str = "Share\u{2026}";
/// What `Unpublished` says in the desktop's words: `guidance()` names the
/// CLI's `item sync`.
pub(crate) const UNPUBLISHED: &str =
    "This item is not shared yet. Share\u{2026} publishes it to the shared copy.";

/// What a listed line offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineAction {
    /// Share the item (`item sync`), as its header's Share does.
    Share,
}

impl LineAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Share => SHARE,
        }
    }
}

/// One item the poll left alone, and what to say of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EntryLine {
    pub item_id: Option<ItemId>,
    pub kind: Option<TargetKind>,
    pub text: &'static str,
    /// Only where the item can be acted on: an item without an ID or a
    /// kind offers none.
    pub action: Option<LineAction>,
}

/// Where a retirement of one item stands (E3, "what the person sees").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RetireState {
    /// The retirement is queued or running.
    Retiring,
    /// `retired` or `already_absent`: the copy is gone.
    Retired,
    /// A Resume finished the row the retirement left (its index hand-off):
    /// it is over, and nothing says the copy was removed by the person's
    /// request.
    Finished,
    /// `pending`, or left `Accepted`: it did not finish. The pending row
    /// offers Resume and Abandon (E4).
    Unfinished {
        /// `discovery_pending`: the lists are still catching up.
        catching_up: bool,
    },
}

/// One item's retirement in the attempt's entry. It is the item's ID and
/// the library's fixed words, never a title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetireLine {
    pub kind: TargetKind,
    pub item: ItemId,
    pub state: RetireState,
}

/// A reported `Closed` or `Promoted` item the loop did not retire, which
/// Retire… (E4) takes up: skipped, refused, preserved, failed before any
/// effect, or not reached. Its ID and kind, never a title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetireOffer {
    pub kind: TargetKind,
    pub item: ItemId,
}

/// The item by its short code: the last eight characters of its ULID (E3).
pub(crate) fn short_code(item: &ItemId) -> String {
    let id = item.to_string();
    id.get(id.len().saturating_sub(8)..)
        .unwrap_or(&id)
        .to_owned()
}

/// What the entry says of an item whose retirement it offers: the item by
/// its short code, never a title.
pub(crate) fn offer_text(kind: TargetKind, item: &str) -> String {
    let what = match kind {
        TargetKind::Ticket => "is closed",
        TargetKind::Document => "was promoted",
    };
    format!("{item} {what}. Its copy on this computer was not retired.")
}

/// "Retire…", the action of such an item.
pub(crate) const RETIRE: &str = "Retire\u{2026}";

impl RetireOffer {
    pub(crate) fn text(&self) -> String {
        offer_text(self.kind, &short_code(&self.item))
    }
}

/// "Retiring {item}…".
pub(crate) fn retiring_text(item: &str) -> String {
    format!("Retiring {item}\u{2026}")
}

/// "Finished retiring {item}.", for a row a Resume finished.
pub(crate) fn finished_text(item: &str) -> String {
    format!("Finished retiring {item}.")
}

/// "Retired the copy of {item} on this computer."
pub(crate) fn retired_text(item: &str) -> String {
    format!("Retired the copy of {item} on this computer.")
}

/// "Retiring {item} didn't finish.", and, when the index is behind, "The
/// lists are still catching up.".
pub(crate) fn unfinished_text(item: &str, catching_up: bool) -> String {
    let said = format!("Retiring {item} didn't finish.");
    if catching_up {
        format!("{said} The lists are still catching up.")
    } else {
        said
    }
}

impl RetireLine {
    /// What the line says: the item by its short ID, the last eight
    /// characters of its ULID.
    pub(crate) fn text(&self) -> String {
        let item = short_code(&self.item);
        match self.state {
            RetireState::Retiring => retiring_text(&item),
            RetireState::Retired => retired_text(&item),
            RetireState::Finished => finished_text(&item),
            RetireState::Unfinished { catching_up } => unfinished_text(&item, catching_up),
        }
    }
}

/// A repository's attempt entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttemptEntry {
    /// "Brought in N changes", when something was.
    pub brought_in: Option<String>,
    pub lines: Vec<EntryLine>,
    /// The retirements since the attempt, in the order they began (E3).
    pub retirements: Vec<RetireLine>,
    /// The items Retire… is offered for (E4), in the poll's order.
    pub offers: Vec<RetireOffer>,
}

impl AttemptEntry {
    /// The entry with `offers` added. An attempt that had nothing else to
    /// say still has an entry once there is an item to retire.
    pub(crate) fn with_offers(entry: Option<Self>, offers: Vec<RetireOffer>) -> Option<Self> {
        if offers.is_empty() {
            return entry;
        }
        let mut entry = entry.unwrap_or(Self {
            brought_in: None,
            lines: Vec::new(),
            retirements: Vec::new(),
            offers: Vec::new(),
        });
        entry.offers = offers;
        Some(entry)
    }
}

/// The library's words for `disposition`; `None` for the ones this entry
/// does not list (`Closed`, `Promoted`: Part E; the ones that are counts).
fn text_of(disposition: Disposition) -> Option<&'static str> {
    use Disposition::*;
    let library = match disposition {
        Unpublished => return Some(UNPUBLISHED),
        Diverged => PollDisposition::Diverged,
        Dirty => PollDisposition::Dirty,
        Conflicted => PollDisposition::Conflicted,
        Inaccessible => PollDisposition::Inaccessible,
        Nonconforming => PollDisposition::Nonconforming,
        BlockedByLocalState => PollDisposition::BlockedByLocalState,
        MaterializationIncomplete => PollDisposition::MaterializationIncomplete,
        Malformed => PollDisposition::Malformed,
        RemotelyDeleted => PollDisposition::RemotelyDeleted,
        HistoryUnknown => PollDisposition::HistoryUnknown,
        NotReached => PollDisposition::NotReached,
        Current | FastForwarded | LocalAhead | Materialized | Closed | Promoted => return None,
    };
    Some(library.guidance())
}

/// The entry `summary` earns, if it has anything to say: what was brought
/// in, and a line for each item the poll left alone, in the poll's order.
/// The retirement summary and the failure slots join the entry in Tasks 11a
/// and 12, each adding to what makes it worth showing.
pub(crate) fn attempt_entry(
    summary: &AttemptSummary,
    retirements: &[RetireLine],
) -> Option<AttemptEntry> {
    let brought_in = (summary.brought_in > 0).then(|| changes(summary.brought_in));
    let lines: Vec<EntryLine> = summary
        .listed
        .iter()
        .filter_map(|target| {
            let text = text_of(target.disposition)?;
            let action = (target.disposition == Disposition::Unpublished
                && target.item_id.is_some()
                && target.kind.is_some())
            .then_some(LineAction::Share);
            Some(EntryLine {
                item_id: target.item_id.clone(),
                kind: target.kind,
                text,
                action,
            })
        })
        .collect();
    (brought_in.is_some() || !lines.is_empty() || !retirements.is_empty()).then(|| AttemptEntry {
        brought_in,
        lines,
        retirements: retirements.to_vec(),
        offers: Vec::new(),
    })
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod tests;
