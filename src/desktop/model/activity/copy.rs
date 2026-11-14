//! Part B's words: every user-visible string Activity's rows say, as a
//! constant, or a formatting function for a string with values. The pure
//! text helpers the old operation area built (`outcome_label`,
//! `effects_text`, `scope_name`, `SHARE_MAIN`) live here too, so the model
//! reads no view module (R-41); each is re-exported from its old path.

mod states;

use std::path::Path;

use manyhands::{
    canonical::ItemId,
    repository::AuthoringKind,
    results::{Effects, Outcome},
};

pub(crate) use self::states::*;
use super::rows::Derived;
use crate::desktop::model::{
    item::SHARE,
    list_view::RETRY,
    operations::{ANOTHER_SHARE_HOLDS, HISTORY_UNRESOLVED_GUIDANCE, stopped_text},
    polling::entry::short_code,
    publication::{ShareTarget, SyncOffer, SyncOutcome},
};

/// A control a request row offers (B2).
pub(crate) const CANCEL: &str = "Cancel";
/// An accepted share's Stop (R-1).
pub(crate) const STOP: &str = "Stop at a safe point";
/// Stop, once pressed.
pub(crate) const STOPPING: &str = "Stopping\u{2026}";
/// An accepted request that cannot be stopped.
pub(crate) const FINISHING: &str = "Finishing";
pub(crate) const DISMISS: &str = "Dismiss";
/// A lifecycle's entry, once failed or stopped: its dialog again (D5, A3).
pub(crate) const OPEN_LIFECYCLE: &str = "Open\u{2026}";
/// The entry points of the conflict sheet (B5): the operation area's, a
/// pending row's and the banner's.
pub(crate) const REVIEW_CHANGES: &str = "Review changes";

/// A request's state in the tooltip's words.
pub(crate) const QUEUED: &str = "Queued";
pub(crate) const RUNNING: &str = "Running";
pub(crate) const CANCELLED: &str = "Cancelled";
pub(crate) const NOT_SENT: &str = "Not sent";

/// The share offers' labels (R-44: today's, verbatim).
pub(crate) const UNLOCK: &str = "Unlock\u{2026}";
/// Give up (decision 8).
pub(crate) const GIVE_UP: &str = "Stop trying to share";
pub(crate) const OPEN_SHARING_AND_KEYS: &str = "Open Sharing & keys";
pub(crate) const CHECK_HOST: &str = "Check host";
pub(crate) const SET_IDENTITY: &str = "Set identity";
pub(crate) const SHOW_IT: &str = "Show it";
pub(crate) const SET_UP_SHARING: &str = "Set up sharing";
pub(crate) const REPUBLISH: &str = "Republish\u{2026}";
pub(crate) const STOP_AND_REPUBLISH: &str = "Stop and republish\u{2026}";
/// A share's deleted branch whose republish is queued or running.
pub(crate) const REPUBLISHING: &str = "Republishing\u{2026}";
/// A Give up that found its request's record already finished: nothing was
/// cancelled, and the entry shows the stored answer (decision 8).
pub(crate) const ALREADY_FINISHED: &str = "Already finished";
/// An retirement answered Owed whose lists are behind (A2, R-51).
pub(crate) const LISTS_CATCHING_UP: &str = "The lists are still catching up.";
/// What B2 says of a finished request that changed nothing.
pub(crate) const NOTHING_TO_CHANGE: &str = "Nothing needed to change.";
/// "Your work is safe" ends a failure's description (R-31).
const WORK_IS_SAFE: &str = "Your work is safe on this computer.";

/// What a request or operation is about when it names no repository: a
/// key command's scope.
pub(crate) const APPLICATION: &str = "Application";

/// The repository menu's Share main: `repo sync` (design, Lane B, Share
/// controls).
pub(crate) const SHARE_MAIN: &str = "Share main";

/// An outcome in fixed text.
pub(crate) fn outcome_label(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Success => "Done",
        Outcome::Noop => "No change",
        Outcome::Partial => "Partly done",
        Outcome::Blocked => "Blocked",
        Outcome::Cancelled => "Cancelled",
        Outcome::Error => "Failed",
    }
}

/// What a request or operation is about: its repository's directory
/// name, or the application for a key command.
pub(crate) fn scope_name(root: Option<&Path>) -> String {
    match root {
        Some(root) => root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned()),
        None => APPLICATION.to_owned(),
    }
}

/// The effects a request reports, each one it was asked for, in the
/// contract's names.
pub(crate) fn effects_text(effects: &Effects) -> String {
    let named = [
        ("write", effects.write.as_str()),
        ("checkpoint", effects.checkpoint.as_str()),
        ("discovery", effects.discovery.as_str()),
        ("publication", effects.publication.as_str()),
        ("integration", effects.integration.as_str()),
        ("cleanup", effects.cleanup.as_str()),
    ];
    let shown: Vec<String> = named
        .into_iter()
        .filter(|(_, value)| *value != "not_requested")
        .map(|(name, value)| format!("{name}: {value}"))
        .collect();
    if shown.is_empty() {
        "No effects".to_owned()
    } else {
        shown.join(", ")
    }
}

/// Nothing in the list yet: Activity's empty state (C1).
pub(crate) const NOTHING_YET: &str = "Nothing yet this session";

/// Activity's empty state with the filter on (C1).
pub(crate) const NOTHING_NEEDS_ATTENTION: &str = "Nothing needs attention";

/// The filter button's accessible name and tooltip (`03`, C1).
pub(crate) const SHOW_ONLY_ATTENTION: &str = "Show only things that need attention";

/// The one item of Activity's ⋯ menu (C1, L8).
pub(crate) const CLEAR_FINISHED: &str = "Clear finished";

/// The Needs attention popover's header (E2, `05`).
pub(crate) const NEEDS_ATTENTION_TITLE: &str = "Needs attention";

/// Under the popover's header: nothing is lost, and nothing happens by
/// itself (E2, `05`).
pub(crate) const NEEDS_ATTENTION_SAFE: &str =
    "Your work is safe on this computer. Nothing here happens until you choose.";

/// The popover's link to Activity (E2).
pub(crate) const SHOW_EVERYTHING: &str = "Show everything in Activity";

/// A row's full message (B5): its title, then ". " and its description when
/// it has one. Activity draws it on one line, and the row menu's first item
/// is it.
pub(crate) fn message_of(row: &Derived) -> String {
    match &row.description {
        Some(description) => format!("{}. {description}", row.title),
        None => row.title.clone(),
    }
}

/// A row's tooltip, line by line (B5): the full message, the time in full,
/// then the row's detail. `time` is B6's tooltip time. The same text is the
/// row's accessible description.
pub(crate) fn tooltip_lines(row: &Derived, time: &str) -> Vec<String> {
    let mut lines = vec![message_of(row), time.to_owned()];
    lines.extend(row.detail.iter().cloned());
    lines
}

/// An offer's label.
pub(crate) fn offer_label(offer: SyncOffer) -> &'static str {
    match offer {
        SyncOffer::Share => SHARE,
        SyncOffer::Retry => RETRY,
        SyncOffer::GiveUp => GIVE_UP,
        SyncOffer::Unlock => UNLOCK,
        SyncOffer::OpenSharingAndKeys => OPEN_SHARING_AND_KEYS,
        SyncOffer::CheckHost => CHECK_HOST,
        SyncOffer::SetIdentity => SET_IDENTITY,
        SyncOffer::ShowHolder => SHOW_IT,
        SyncOffer::SetUpSharing => SET_UP_SHARING,
        SyncOffer::Republish => REPUBLISH,
        SyncOffer::StopAndRepublish => STOP_AND_REPUBLISH,
    }
}

/// An item as a row names it (B1): “{title}” in typographic quotes when a
/// title was read, else "the ticket {short}" or "the document {short}". Never
/// a draft's own title.
pub(crate) fn item_name(title: Option<&str>, kind: AuthoringKind, id: &ItemId) -> String {
    match (title, kind) {
        (Some(title), _) => format!("\u{201c}{title}\u{201d}"),
        (None, AuthoringKind::Ticket) => format!("the ticket {}", short_code(id)),
        (None, AuthoringKind::Document) => format!("the document {}", short_code(id)),
    }
}

/// "{Failure} {work is safe}": a failed request's description (B2).
pub(crate) fn failure_description(message: &str) -> String {
    format!("{message} {WORK_IS_SAFE}")
}

/// The lines of a lifecycle's outcome as a description: one sentence each,
/// joined, ending with a full stop (B2).
pub(crate) fn lines_description(lines: &[String]) -> String {
    let sentences: Vec<&str> = lines
        .iter()
        .map(|line| line.trim_end_matches('.'))
        .collect();
    format!("{}.", sentences.join(". "))
}

/// A sentence that ends with a full stop (a description ends with one).
fn sentence(words: &str) -> String {
    format!("{}.", words.trim_end_matches('.'))
}

/// A request's title with its first letter lower case, for "Stopped …".
pub(crate) fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

/// What a finished share says (B3): its kind as a row's, its title and its
/// description.
pub(crate) struct ShareWords {
    pub kind: super::rows::Kind,
    pub title: String,
    pub description: Option<String>,
}

/// B3's table. `subject` is “{title}” for an item, or the main branch for
/// main; `repo` the repository's name; `holder` the Busy holder's words the
/// Workspace worked out (R-9); `failure` the failed answer's description.
pub(crate) fn share_words(
    outcome: &SyncOutcome,
    target: ShareTarget,
    subject: &str,
    repo: &str,
    holder: Option<&str>,
    failure: &str,
) -> ShareWords {
    use super::rows::Kind::{Failure, Info, Locked, Sharing, Success};
    let (kind, title, description) = match outcome {
        SyncOutcome::Published => (Sharing, format!("Shared {subject}"), None),
        SyncOutcome::Current => (
            Success,
            // An item named by its short code begins a title with a capital,
            // as the state rows do (T4-2); the main branch keeps its name.
            match target {
                ShareTarget::Item => {
                    format!("{} is the same as the shared copy", upper_first(subject))
                }
                ShareTarget::Main => format!("{subject} is the same as the shared copy"),
            },
            None,
        ),
        SyncOutcome::Stopped { completed } => (
            Info,
            format!("Stopped sharing {subject}"),
            Some(stopped_text(*completed).to_owned()),
        ),
        SyncOutcome::Conflict { .. } => (
            Failure,
            format!("You and someone else both changed {subject}"),
            Some("Choose what to keep. Nothing is shared until you do.".to_owned()),
        ),
        SyncOutcome::BranchDeleted => (
            Failure,
            format!("The shared copy of {subject} was deleted"),
            Some("Republish it to share it again.".to_owned()),
        ),
        SyncOutcome::HistoryUnresolved => (
            Failure,
            format!("Couldn't confirm the shared history of {subject}"),
            Some(HISTORY_UNRESOLVED_GUIDANCE.to_owned()),
        ),
        SyncOutcome::NeedsUnlock => (
            Locked,
            format!("Couldn't share {subject}: your key is locked"),
            Some("Unlock it, then share again.".to_owned()),
        ),
        SyncOutcome::NeedsKey => (
            Failure,
            format!("Couldn't share {subject}: choose a key to share with"),
            None,
        ),
        SyncOutcome::NeedsHost { authority } => {
            let host = authority.as_ref().map_or("the host", |a| a.host.as_str());
            (
                Failure,
                format!("Couldn't share {subject}: check {host} before sharing"),
                None,
            )
        }
        SyncOutcome::NeedsIdentity => (
            Failure,
            format!("Couldn't share {subject}: set your name first"),
            None,
        ),
        SyncOutcome::Busy => (
            Failure,
            format!("Couldn't share {subject} yet"),
            Some(sentence(holder.unwrap_or(ANOTHER_SHARE_HOLDS))),
        ),
        SyncOutcome::NoRemote => (
            Info,
            format!("{repo} has nowhere to share {subject} to"),
            None,
        ),
        SyncOutcome::NothingToShare => (
            Info,
            format!("Nothing to share in {subject} yet"),
            Some("It has no saved changes.".to_owned()),
        ),
        SyncOutcome::Failed { .. } => (
            Failure,
            format!("Couldn't share {subject}"),
            Some(failure.to_owned()),
        ),
    };
    ShareWords {
        kind,
        title,
        description,
    }
}
