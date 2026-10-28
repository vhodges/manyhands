//! An item tab's edit session as pure decisions (design, Part C, Phases):
//! where the tab stands, whether Edit is offered, the base a session's
//! draft begins on, and what the toolbar's status item says of D2's
//! recovery copy.
//!
//! Nothing here holds the item's text but `fields_with_body`'s answer, a
//! draft's fields, which derive no `Debug`.

use manyhands::{
    drafts::{DraftBase, DraftFields},
    repository::{ClosureState, ItemDto},
};
use time::OffsetDateTime;

use super::{editor_text::normalized, navigation::OpenTarget};
use crate::desktop::services::requests::DesktopRequestId;

/// The header's Edit while a draft of the item is kept or stored (R-35).
pub(crate) const CONTINUE_EDITING: &str = "Continue editing";
/// The header's way back to viewing while editing.
pub(crate) const DONE: &str = "Done";

/// The toolbar's status words (design, The toolbar).
pub(crate) const DRAFT_NOT_KEPT: &str = "Draft not kept. Save to protect your changes";
pub(crate) const RECOVERY_OFF: &str = "Crash recovery is off";

/// Where an item tab stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    /// The read editor shows the item.
    Viewing,
    /// `context prepare` runs as this request.
    Preparing(DesktopRequestId),
    /// The context is ready; the item is read again before the editor opens
    /// (R-70). After a `discovery_pending` answer, the tab waits here for
    /// Resume (R-13, R-77).
    Loading,
    /// The edit editor holds the item.
    Editing,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 15.
    Saving,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 19.
    Review,
}

/// Whether Edit is offered for `item`, read in a tab of `target`: an
/// item's own tab, never an Inspect tab, which stays the read-only source
/// view (R-64); a conforming item whose body was read; not a closed ticket.
pub(crate) fn edit_offered(target: &OpenTarget, item: &ItemDto) -> bool {
    matches!(target, OpenTarget::Item { .. })
        && item.id.is_some()
        && item.body.is_some()
        && !item
            .closure
            .as_ref()
            .is_some_and(|closure| closure.state == ClosureState::Closed)
}

/// The base an Edit's draft begins on (R-11): the item as read, with its
/// body as the editor loads it and a ticket's `deps` in ID order (R-72).
/// `None` for a nonconforming entry.
pub(crate) fn base_for_edit(item: &ItemDto) -> Option<DraftBase> {
    DraftBase::of(&normalized(item))
}

/// `fields` with `body` as its body: the draft of an editor's text over
/// its base. A comment's fields are not an item's, and stay as they are.
pub(crate) fn fields_with_body(fields: &DraftFields, body: String) -> DraftFields {
    let mut fields = fields.clone();
    match &mut fields {
        DraftFields::Document { body: old, .. } | DraftFields::Ticket { body: old, .. } => {
            *old = body;
        }
        DraftFields::Comment { .. } => {}
    }
    fields
}

/// What D2 says of a session's recovery copy: the toolbar status item's
/// only input. It holds no text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DraftState {
    /// The draft store did not open.
    StoreUnavailable,
    /// No draft has begun, or its first write is still in flight: nothing
    /// is claimed.
    NotBegun,
    /// The draft could not be begun, or its last write failed.
    Failed,
    /// The newest text, or an older revision, was last written at this
    /// time.
    Written(OffsetDateTime),
}

/// The toolbar's status item (design, The toolbar).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DraftIndicator {
    /// "Draft kept · {n}s ago": the last write landed `age_secs` ago.
    Kept { age_secs: u64 },
    /// "Draft not kept. Save to protect your changes".
    NotKept,
    /// "Crash recovery is off".
    RecoveryOff,
}

/// The indicator for D2's `state` at `now`. It never claims a write that
/// did not happen: the age is the last write's, and nothing is said before
/// the first write lands.
pub(crate) fn draft_indicator(state: DraftState, now: OffsetDateTime) -> Option<DraftIndicator> {
    match state {
        DraftState::StoreUnavailable => Some(DraftIndicator::RecoveryOff),
        DraftState::NotBegun => None,
        DraftState::Failed => Some(DraftIndicator::NotKept),
        DraftState::Written(at) => {
            let age = (now - at).whole_seconds().max(0);
            Some(DraftIndicator::Kept {
                age_secs: u64::try_from(age).unwrap_or(0),
            })
        }
    }
}

/// The indicator in words.
pub(crate) fn indicator_words(indicator: DraftIndicator) -> String {
    match indicator {
        DraftIndicator::Kept { age_secs } => format!("Draft kept \u{b7} {age_secs}s ago"),
        DraftIndicator::NotKept => DRAFT_NOT_KEPT.to_owned(),
        DraftIndicator::RecoveryOff => RECOVERY_OFF.to_owned(),
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
