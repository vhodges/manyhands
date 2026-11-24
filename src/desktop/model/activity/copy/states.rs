//! Part B4's words: what the state rows (S2-S14) say. A constant, or a
//! formatting function for a string with values; sentence case, a title
//! never ends with a full stop and a description always does.
//!
//! No row names a draft's own title or a path (decision 21), and nothing
//! here shows a repository's folder or the shared copy's location (R-48).

use manyhands::repository::{OperationAction, OperationNextAction};

use crate::desktop::model::{
    polling::{Disposition, Reason, entry::text_of},
    repository_state::{RepoState, sentence_line},
};

/// Open, a problem's button.
pub(crate) const OPEN: &str = "Open";
/// A pending row's Resume, and a lifecycle parent's.
pub(crate) const RESUME: &str = "Resume";
/// A pending local operation's Abandon: today's label (R-44).
pub(crate) const ABANDON: &str = "Abandon";
/// The recovered-drafts row's Review: today's (R-44).
pub(crate) const REVIEW: &str = "Review";
/// Share or Share main while a share of the same target runs.
pub(crate) const SHARING: &str = "Sharing\u{2026}";

/// Why S3's Unlock… is disabled: today's words (`STEP_ASIDE`), as a sentence.
pub(crate) const UNLOCK_UNAVAILABLE: &str =
    "Finish the pending operations of these repositories first.";
/// What a retirement that did not finish says of what happens next.
pub(crate) const RETIRE_TRIED_AGAIN: &str = "It is tried again at the next check.";
/// An action with no entry in `action_words`.
pub(crate) const AN_OPERATION: &str = "An operation";
/// A pending row that names no operation.
pub(crate) const NO_OPERATION: &str = "No operation ID";

/// The kind of row a draft or a note is, for its tooltip (B5).
pub(crate) const DETAIL_FLUSH: &str = "Recovery copy";
pub(crate) const DETAIL_STORE: &str = "Recovery store";
pub(crate) const DETAIL_RECOVERED: &str = "Recovered drafts";
pub(crate) const DETAIL_NOTE: &str = "Record note";

/// S4's descriptions that are not Cycle 03a's sentence.
pub(crate) const CANT_REACH_DESCRIPTION: &str = "Your work is safe, and Manyhands tries again.";
pub(crate) const LOCKED_DESCRIPTION: &str = "Check now asks for it.";

fn count(count: u32, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        count => format!("{count} {many}"),
    }
}

/// The first letter upper case, for a name that begins a title.
pub(crate) fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// A sentence without its full stop, for a title (a title never ends with
/// one).
pub(crate) fn title_of(sentence: &str) -> String {
    sentence.trim_end_matches('.').to_owned()
}

/// A state's own sentence as a description: its parts joined, ending with a
/// full stop. `None` when it says nothing.
pub(crate) fn description_of(state: &RepoState) -> Option<String> {
    let line = sentence_line(state);
    if line.is_empty() {
        return None;
    }
    Some(if line.ends_with(['.', '!', '?']) {
        line
    } else {
        format!("{line}.")
    })
}

/// "Main branch: {primary}" and "Shared copy: {remote}": the repository's
/// tooltip lines in Activity, the remote by name only and never the folder
/// or the location (R-48).
pub(crate) fn repository_detail(primary: Option<&str>, remote: Option<&str>) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(primary.map(crate::desktop::model::repository_state::main_branch_line));
    lines.extend(remote.map(|name| format!("Shared copy: {name}")));
    lines
}

/// S2, an automatic check: "Checked {repo} for updates".
pub(crate) fn checked_title(repo: &str) -> String {
    format!("Checked {repo} for updates")
}

/// S2's description: "Brought in 1 change." or "Brought in {n} changes."
pub(crate) fn brought_description(changes: u32) -> String {
    format!("Brought in {}.", count(changes, "change", "changes"))
}

/// S2, a Check now: "Brought in 1 change to {repo}".
pub(crate) fn brought_title(changes: u32, repo: &str) -> String {
    format!(
        "Brought in {} to {repo}",
        count(changes, "change", "changes")
    )
}

/// S3's title and description for a hold over `repos`, the held
/// repositories' names joined with ", ".
pub(crate) fn hold_words(reason: &Reason, repos: &str) -> (String, String) {
    let waits = format!("Checking {repos} waits until you approve it.");
    match reason {
        Reason::KeyLocked(_) => (
            "Your key is locked".to_owned(),
            format!("Checking {repos} is waiting for the passphrase."),
        ),
        Reason::KeyProblem(_) | Reason::NoKeySelected(_) => (
            "Checking needs a key to be chosen".to_owned(),
            format!("Holding back {repos}."),
        ),
        Reason::HostUnapproved(authority) => {
            (format!("{} needs your approval", authority.host), waits)
        }
        Reason::HostUnidentified(_) => ("A server needs your approval".to_owned(), waits),
        Reason::ConfigurationRequired(_) => (
            format!("Sharing isn't set up for {repos}"),
            "Checking can't start until it is.".to_owned(),
        ),
        Reason::RecoveryHeld(_) => (
            format!("Checking {repos} is held until recovery"),
            "Your work is safe.".to_owned(),
        ),
    }
}

pub(crate) fn cant_open_title(repo: &str) -> String {
    format!("Can't open {repo}")
}

pub(crate) fn needs_fixing_title(repo: &str) -> String {
    format!("{repo} needs fixing")
}

pub(crate) fn cant_reach_title(repo: &str) -> String {
    format!("Couldn't reach {repo}'s shared copy")
}

pub(crate) fn locked_title(repo: &str) -> String {
    format!("Checking {repo} needs your key's passphrase")
}

pub(crate) fn needs_attention_title(repo: &str) -> String {
    format!("Checking {repo} needs attention")
}

pub(crate) fn no_shared_copy_title(repo: &str) -> String {
    format!("{repo} has no shared copy, so changes stay on this computer")
}

/// How main is named in a title: "{primary} in {repo}".
pub(crate) fn main_in(primary: &str, repo: &str) -> String {
    format!("{primary} in {repo}")
}

/// How an item whose kind is not known is named: its short code.
pub(crate) fn item_of_unknown_kind(short: &str) -> String {
    format!("the item {short}")
}

/// How an item whose ID did not parse is named.
pub(crate) const UNNAMED_ITEM: &str = "an item";

/// S5: "{name} needs a look".
pub(crate) fn problem_title(name: &str) -> String {
    format!("{name} needs a look")
}

/// S6: "{name} isn't shared yet".
pub(crate) fn unshared_title(name: &str) -> String {
    format!("{name} isn't shared yet")
}

/// S5's sentence for a disposition that needs a look; `None` for any other.
pub(crate) fn disposition_sentence(disposition: Disposition) -> Option<&'static str> {
    use Disposition::*;
    Some(match disposition {
        Dirty => "It has changes on this computer that aren't saved yet.",
        Conflicted => "It has a conflict to resolve.",
        Inaccessible => "Manyhands can't read it.",
        Nonconforming => "It needs fixing before it can be shared.",
        BlockedByLocalState => "Something on this computer is in the way of updating it.",
        MaterializationIncomplete => "Bringing it in didn't finish.",
        Malformed => "Its file can't be read as a ticket or document.",
        RemotelyDeleted => "Its shared copy was deleted.",
        HistoryUnknown => "Manyhands can't confirm its shared history.",
        NotReached => "The check didn't reach it this time.",
        Current | FastForwarded | LocalAhead | Materialized | Closed | Promoted | Unpublished
        | Diverged => return None,
    })
}

/// A disposition named in a tooltip.
pub(crate) fn disposition_name(disposition: Disposition) -> &'static str {
    use Disposition::*;
    match disposition {
        Current => "Current",
        FastForwarded => "Fast-forwarded",
        LocalAhead => "Local ahead",
        Diverged => "Diverged",
        Dirty => "Dirty",
        Conflicted => "Conflicted",
        Inaccessible => "Inaccessible",
        Materialized => "Materialized",
        Nonconforming => "Nonconforming",
        BlockedByLocalState => "Blocked by local state",
        MaterializationIncomplete => "Materialization incomplete",
        Malformed => "Malformed",
        RemotelyDeleted => "Remotely deleted",
        Closed => "Closed",
        Promoted => "Promoted",
        HistoryUnknown => "History unknown",
        Unpublished => "Unpublished",
        NotReached => "Not reached",
    }
}

/// S5's tooltip line: "{disposition}: {library guidance}" (fact 20).
pub(crate) fn problem_detail(disposition: Disposition) -> String {
    let name = disposition_name(disposition);
    match text_of(disposition) {
        Some(guidance) => format!("{name}: {guidance}"),
        None => name.to_owned(),
    }
}

/// S6's tooltip line: "Not shared yet: {disposition}".
pub(crate) fn unshared_detail(disposition: Disposition) -> String {
    format!("Not shared yet: {}", disposition_name(disposition))
}

/// S7: a ticket that closed, or a document that was promoted, whose copy
/// is left.
pub(crate) fn retire_offer_title(name: &str, ticket: bool) -> String {
    let what = if ticket { "is closed" } else { "is approved" };
    format!("{name} {what}, but its copy is still on this computer")
}

/// S8, a retired copy.
pub(crate) fn retired_title(name: &str) -> String {
    format!("Retired the copy of {name} on this computer")
}

/// S8, a retirement that did not finish.
pub(crate) fn retire_note_title(name: &str) -> String {
    format!("Retiring {name} didn't finish")
}

/// S9's words for each action the library lists (R-32). `{primary}` is the
/// repository's main branch.
const ACTION_WORDS: [(OperationAction, &str); 23] = [
    (OperationAction::CreateAndEnable, "Adding a repository"),
    (OperationAction::Enable, "Adding a repository"),
    (OperationAction::RemoveRegistration, "Removing a repository"),
    (OperationAction::AddRemote, "Adding a shared copy"),
    (OperationAction::RemoveRemote, "Removing a shared copy"),
    (
        OperationAction::SetPublicationRemote,
        "Choosing how it shares",
    ),
    (OperationAction::Refresh, "Refreshing"),
    (OperationAction::Rebuild, "Rebuilding the search index"),
    (
        OperationAction::PrepareContext,
        "Getting a private copy ready",
    ),
    (
        OperationAction::ReleaseContext,
        "Putting away a private copy",
    ),
    (OperationAction::RetireContext, "Retiring a copy"),
    (OperationAction::SaveDocument, "Saving a document"),
    (OperationAction::SaveTicket, "Saving a ticket"),
    (OperationAction::SubmitComment, "Posting a comment"),
    (OperationAction::RepairItem, "Repairing an item"),
    (
        OperationAction::ContextSyncLocal,
        "Repairing a copy on this computer",
    ),
    (OperationAction::Poll, "Checking for updates"),
    (OperationAction::SynchronizeContext, "Sharing"),
    (OperationAction::SynchronizePrimary, "Sharing {primary}"),
    (OperationAction::Promote, "Approving and merging"),
    (OperationAction::Close, "Closing"),
    (OperationAction::GenerateKey, "Creating a key"),
    (OperationAction::DeleteKey, "Deleting a key"),
];

/// S9's words for an action; "An operation" for one with no entry.
pub(crate) fn action_words(action: OperationAction, primary: &str) -> String {
    ACTION_WORDS
        .iter()
        .find(|(listed, _)| *listed == action)
        .map_or(AN_OPERATION, |(_, words)| words)
        .replace("{primary}", primary)
}

/// S9's title: "{words} “{title}” didn't finish" when the item is known,
/// else "{words} didn't finish in {repo}".
pub(crate) fn pending_title(words: &str, item: Option<&str>, repo: &str) -> String {
    match item {
        Some(name) => format!("{words} {name} didn't finish"),
        None => format!("{words} didn't finish in {repo}"),
    }
}

/// S9's description, by the next action the operation waits for.
pub(crate) fn next_action_sentence(next: Option<OperationNextAction>) -> &'static str {
    match next {
        Some(OperationNextAction::Resume) => "Resume finishes it.",
        Some(OperationNextAction::RetryGeneration) => "Resume tries again.",
        Some(OperationNextAction::ReviewDeletionAgain) => "Review it again.",
        Some(OperationNextAction::InspectRetainedFiles) => {
            "Some files were kept for you to look at."
        }
        None => "Nothing to do.",
    }
}

/// S10, a pending list that could not be read.
pub(crate) fn pending_list_failed(repo: &str) -> String {
    format!("Couldn't read the unfinished operations of {repo}")
}

/// S10, a pending list that was busy.
pub(crate) fn pending_list_busy(repo: &str) -> String {
    format!("Manyhands was busy reading the unfinished operations of {repo}")
}

/// S11: "The recovery copy of {name} wasn't saved".
pub(crate) fn flush_title(name: &str) -> String {
    format!("The recovery copy of {name} wasn't saved")
}

/// S13: "1 draft recovered from an earlier session" and its plural.
pub(crate) fn recovered_title(drafts: usize) -> String {
    let drafts = u32::try_from(drafts).unwrap_or(u32::MAX);
    format!(
        "{} recovered from an earlier session",
        count(drafts, "draft", "drafts")
    )
}
