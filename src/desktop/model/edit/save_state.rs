//! The save states (design, Save states; contract 8, R-14): what a save's
//! answer, or a session about to save, comes to, and the plain words the
//! tab says of it (`06-language-and-states.md`). The pill's words are the
//! pill's (`view::item::pill`); the banners' and the actions' are here.
//!
//! Nothing here holds text the user wrote.

use manyhands::results::{CheckpointEffect, Effects, PublicationEffect, ResultCode};

/// A hand-off owed, a prepare's or a save's: finish it.
pub(crate) const RESUME: &str = "Resume";
/// The same request again.
pub(crate) const RETRY: &str = "Retry";
/// The header's Save while editing.
pub(crate) const SAVE: &str = "Save";
/// Save's tooltip (design, Header block).
pub(crate) const SAVE_TOOLTIP: &str = "Save a checkpoint on this computer (Ctrl S)";
/// Save, on an unverified base: it opens the review instead (R-36).
pub(crate) const REVIEW_BEFORE_SAVING: &str = "Review before saving";
/// The toolbar's status item after `secondary-s` on a clean editor.
pub(crate) const NO_CHANGES_TO_SAVE: &str = "No changes to save";
/// The unverified base's banner action, which opens the review.
pub(crate) const REVIEW: &str = "Review\u{2026}";
/// `discovery_pending`: saved, with the index's hand-off still owed.
pub(crate) const SAVED_INDEX_PENDING: &str =
    "Saved on this computer, but the search index is not updated yet. Nothing is lost.";
/// A pending checkpoint, or an error after acceptance.
pub(crate) const CHECKPOINT_FAILED: &str =
    "Could not save a checkpoint on this computer. Your text is kept.";
/// An unverified base: someone may have changed the item since the save.
pub(crate) const CHECK_CURRENT_VERSION: &str =
    "Check the current version before saving. Your edits are kept.";
/// `external_change`: the handoff's words for the change banner.
pub(crate) const CHANGED_OUTSIDE: &str =
    "Someone changed this file outside Manyhands. Your edits are kept.";
/// Any other refusal, before the library's message and code.
pub(crate) const SAVE_REFUSED: &str = "Could not save on this computer. Your edits are kept.";

/// What a save comes to, a row of the design's Save states table. It
/// holds no text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SaveState {
    /// Dirty, not yet saved.
    Unsaved,
    /// `ok` or `already_applied`, committed or unchanged;
    /// `publication_pending` with a selected publication remote.
    SavedLocally { publication_pending: bool },
    /// Saved, with the index's hand-off owed: Resume.
    DiscoveryPending,
    /// A pending checkpoint, or an error after acceptance: Retry, under the
    /// same request ID.
    CheckpointFailed,
    /// `external_change`: the review opens.
    Changed,
    /// An unverified base: Review….
    ReviewBeforeSaving,
    /// Any other refusal, in the library's message and code.
    Refused(ResultCode),
}

/// The state a save's answer comes to, as D2 reads the same answer
/// (`model::drafts::save::save_verdict`):
/// - `external_change` opens the review;
/// - a pending checkpoint, whatever the code, is kept for Retry;
/// - `discovery_pending` is saved, with the index's hand-off owed;
/// - `ok` and `already_applied`, committed or unchanged, are saved on this
///   computer, not shared yet with a selected publication remote, which a
///   save never publishes to. A publication remote is known only once the
///   remotes are read, after the save (fact 41);
/// - a save that wrote and committed nothing, cancelled or `ok` with no
///   checkpoint, leaves the text unsaved;
/// - anything else is refused.
pub(crate) fn save_state(
    code: ResultCode,
    effects: &Effects,
    has_publication_remote: bool,
) -> SaveState {
    match code {
        ResultCode::ExternalChange => SaveState::Changed,
        _ if effects.checkpoint == CheckpointEffect::Pending => SaveState::CheckpointFailed,
        ResultCode::DiscoveryPending => SaveState::DiscoveryPending,
        ResultCode::Ok | ResultCode::AlreadyApplied
            if matches!(
                effects.checkpoint,
                CheckpointEffect::Committed | CheckpointEffect::Unchanged
            ) =>
        {
            SaveState::SavedLocally {
                publication_pending: has_publication_remote
                    && !matches!(
                        effects.publication,
                        PublicationEffect::Published | PublicationEffect::Current
                    ),
            }
        }
        ResultCode::Ok | ResultCode::AlreadyApplied | ResultCode::Cancelled => SaveState::Unsaved,
        code => SaveState::Refused(code),
    }
}

/// The state of a session about to save: unsaved, or, on an unverified
/// base, to be reviewed first (R-36).
pub(crate) fn before_save(verified: bool) -> SaveState {
    if verified {
        SaveState::Unsaved
    } else {
        SaveState::ReviewBeforeSaving
    }
}

/// The banner's words for `state`; `None` where the pill says it. A
/// refusal's banner adds the library's message and code.
pub(crate) fn save_banner(state: SaveState) -> Option<&'static str> {
    match state {
        SaveState::Unsaved | SaveState::SavedLocally { .. } => None,
        SaveState::DiscoveryPending => Some(SAVED_INDEX_PENDING),
        SaveState::CheckpointFailed => Some(CHECKPOINT_FAILED),
        SaveState::Changed => Some(CHANGED_OUTSIDE),
        SaveState::ReviewBeforeSaving => Some(CHECK_CURRENT_VERSION),
        SaveState::Refused(_) => Some(SAVE_REFUSED),
    }
}

/// The one action the tab offers for `state`, in words; `None` where it
/// offers none, and for a refusal, whose recovery actions are its own.
pub(crate) fn save_offer(state: SaveState) -> Option<&'static str> {
    match state {
        SaveState::Unsaved => Some(SAVE),
        SaveState::DiscoveryPending => Some(RESUME),
        SaveState::CheckpointFailed => Some(RETRY),
        SaveState::ReviewBeforeSaving => Some(REVIEW),
        SaveState::SavedLocally { .. } | SaveState::Changed | SaveState::Refused(_) => None,
    }
}
