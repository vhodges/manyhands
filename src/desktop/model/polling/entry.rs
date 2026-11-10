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
//! `Unpublished` says so and offers Share…. `Closed` and `Promoted` are
//! Part E's (Retire…): they are left out here until that part lists them.
//! The retirement summary, the failure and the attention slots of the entry
//! are Tasks 11a, 12 and 8's.
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

/// A repository's attempt entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttemptEntry {
    /// "Brought in N changes", when something was.
    pub brought_in: Option<String>,
    pub lines: Vec<EntryLine>,
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
pub(crate) fn attempt_entry(summary: &AttemptSummary) -> Option<AttemptEntry> {
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
    (brought_in.is_some() || !lines.is_empty()).then_some(AttemptEntry { brought_in, lines })
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod tests;
