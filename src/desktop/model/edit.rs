//! An item tab's edit session as pure decisions (design, Part C, Phases,
//! and Done and leaving): where the tab stands, whether Edit is offered,
//! the base a session's draft begins on, the base a restored draft goes on
//! over (R-42), what the toolbar's status item says of D2's recovery
//! copy, what Done comes to, and whether an item was never created.
//!
//! Nothing here holds the item's text but `fields_with_body`'s answer, a
//! draft's fields, which derive no `Debug`. What a save comes to is
//! `save_state`.

use manyhands::{
    drafts::{DraftBase, DraftFields},
    repository::{ClosureState, ItemDto},
};
use time::OffsetDateTime;

use super::{editor_text::normalized, navigation::OpenTarget};
use crate::desktop::services::requests::DesktopRequestId;

pub(crate) mod save_state;

/// The header's Edit while a draft of the item is kept or stored (R-35).
pub(crate) const CONTINUE_EDITING: &str = "Continue editing";
/// The header's way back to viewing while editing.
pub(crate) const DONE: &str = "Done";

/// Done's note while the review is open (design, Done and leaving, step
/// 1).
pub(crate) const DONE_IN_REVIEW: &str = "Use these choices or Cancel first";
/// The banner of a session kept after Done (design, Banners; decision 15).
pub(crate) const KEPT_AFTER_DONE: &str =
    "Your unsaved changes are kept as a draft on this computer.";
/// The banner of a session a discard elsewhere ended (design, Banners;
/// R-71).
pub(crate) const DRAFT_DISCARDED: &str = "The draft was discarded.";
/// The change banner's action, which opens the review (R-68).
pub(crate) const COMPARE: &str = "Compare\u{2026}";

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
    /// A save is in flight, or its committed answer settles; the editor
    /// takes typing meanwhile.
    Saving,
    /// The review is open: Save on an unverified base (R-36), or an
    /// `external_change` answer. Save and Done are disabled; the review
    /// itself is Task 19's.
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

/// R-42: the base a restored draft goes on over once Continue editing has
/// read its item again in the context it made ready. Only when the branch
/// alone changed the token: the draft's `base` had an observation, the
/// read's source is the base's, and the read's fields are the base's (as
/// read, or normalized as a base holds them: R-11). Then the base takes
/// the read's observation and context, keeping its own path, source and
/// fields. `None` otherwise, and the review opens.
pub(crate) fn adopted_base(base: &DraftBase, read: &ItemDto) -> Option<DraftBase> {
    base.observation.as_ref()?;
    let observation = read.observation.clone()?;
    if read.source != base.source {
        return None;
    }
    let theirs = base_for_edit(read)?;
    if theirs.fields != base.fields && DraftFields::of(read).as_ref() != Some(&base.fields) {
        return None;
    }
    Some(DraftBase {
        observation: Some(observation),
        context: theirs.context,
        ..base.clone()
    })
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

/// What Done comes to (design, Done and leaving, steps 3 and 5), once a
/// save in flight has committed and the session's text has been written as
/// far as it can be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DoneOutcome {
    /// Not dirty, nothing stored: the session ends.
    End,
    /// Not dirty, with a stored draft (typed, written, then undone): the
    /// draft is discarded, and the session ends.
    Discard,
    /// Dirty, and the draft is written: the session is kept (R-58).
    Keep,
    /// Dirty, and the draft is not written: Save, Discard or Cancel.
    AskNotKept,
    /// A never-created item: Save, Keep draft, Discard or Cancel.
    AskNotCreated,
}

/// Done's rule, in the design's order: a never-created item asks first
/// (step 3); then, after the write (step 4), a clean session ends,
/// discarding what is stored, and a dirty one is kept when its draft is
/// written, and asks otherwise. No typed text is dropped without a choice
/// (R-15, R-73).
pub(crate) fn done_outcome(
    never_created: bool,
    dirty: bool,
    stored: bool,
    written: bool,
) -> DoneOutcome {
    match (never_created, dirty, stored) {
        (true, ..) => DoneOutcome::AskNotCreated,
        (false, false, false) => DoneOutcome::End,
        (false, false, true) => DoneOutcome::Discard,
        (false, true, true) if written => DoneOutcome::Keep,
        (false, true, _) => DoneOutcome::AskNotKept,
    }
}

/// Whether `base` is a new item's (R-20): it has neither an observation
/// nor a source, as only a draft written before its item's create has.
/// Task 22's `is_new_item_base` states the same rule for the library.
pub(crate) fn new_item_base(base: &DraftBase) -> bool {
    base.observation.is_none() && base.source.is_none()
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
