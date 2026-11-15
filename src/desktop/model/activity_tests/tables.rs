//! The tables of Activity's model tests: B2's words by intent, B3's rows
//! by outcome, and the subject each intent makes.

use manyhands::repository::AuthoringKind;

use super::super::rows::{Family, Kind};
use crate::desktop::services::{JobKind, worker::RecordState};

/// B2's table, one row per intent: its name in the fixtures, Doing, Done,
/// Verb and the Done row's own description. The repository is `demo`, the
/// item “Plan”, the remote `origin`, the host `git.example.invalid`, the
/// main branch `trunk`. ItemSync's and RepoSync's Done is B3's.
#[rustfmt::skip]
pub(super) const COPY: &[(&str, &str, &str, &str, &str)] = &[
    ("RepoCreate", "Creating demo", "Created demo", "create demo", ""),
    ("RepoEnable", "Adding demo", "Added demo to Manyhands", "add demo", ""),
    ("RepoRemove", "Removing demo from Manyhands", "Removed demo from Manyhands", "remove demo from Manyhands", "Its files are unchanged."),
    ("RepoIdentitySet", "Setting your name for demo", "Set your name for demo", "set your name for demo", ""),
    ("RemoteAdd", "Adding the shared copy origin to demo", "Added the shared copy origin to demo", "add the shared copy origin", ""),
    ("RemoteRemove", "Removing the shared copy origin", "Removed the shared copy origin from demo", "remove the shared copy origin", ""),
    ("RemoteSelect", "Choosing how demo shares", "demo now shares through origin", "choose how demo shares", ""),
    ("RemoteSelectNone", "Choosing how demo shares", "demo no longer shares", "choose how demo shares", ""),
    ("IndexRefresh", "Refreshing demo", "Refreshed demo", "refresh demo", ""),
    ("IndexRebuild", "Rebuilding the search index for demo", "Rebuilt the search index for demo", "rebuild the search index for demo", ""),
    ("PollOnce", "Checking demo for updates", "Checked demo for updates", "check demo for updates", ""),
    ("PollConfigure", "Changing how often demo is checked", "Changed how often demo is checked", "change how often demo is checked", ""),
    ("PollPause", "Pausing checks of demo", "Paused checking demo for updates", "pause checking demo", ""),
    ("PollResume", "Resuming checks of demo", "Resumed checking demo for updates", "resume checking demo", ""),
    ("RepoSync", "Sharing trunk", "", "share trunk", ""),
    ("FolderCreate", "Creating a folder in demo", "Created a folder in demo", "create a folder in demo", ""),
    ("KeyGenerate", "Creating a key", "Created a key for sharing", "create a key", ""),
    ("KeyImport", "Adding a key", "Added a key for sharing", "add the key", ""),
    ("KeySelect", "Choosing the key for sharing", "Chose the key used for sharing", "choose the key", ""),
    ("KeyClear", "Clearing the key choice", "No key is chosen for sharing now", "clear the key choice", ""),
    ("KeyRemove", "Removing a key from Manyhands", "Removed a key from Manyhands", "remove the key from Manyhands", "Its files are unchanged."),
    ("KeyDelete", "Deleting a key", "Deleted a key", "delete the key", ""),
    ("HostApprove", "Trusting git.example.invalid", "Trusted git.example.invalid for sharing", "trust git.example.invalid", ""),
    ("HostReplace", "Updating the trusted key for git.example.invalid", "Updated the trusted key for git.example.invalid", "update the trusted key for git.example.invalid", ""),
    ("OperationResume", "Finishing an operation in demo", "Finished an operation in demo", "finish the operation in demo", ""),
    ("OperationAbandon", "Abandoning an operation in demo", "Abandoned an operation in demo", "abandon the operation in demo", ""),
    ("ConflictResolve", "Saving your choices in demo", "Resolved a conflict in demo", "resolve the conflict in demo", ""),
    ("LocalContextResolve", "Saving your choices in demo", "Resolved a conflict on this computer in demo", "resolve the conflict in demo", ""),
    ("DocumentSave", "Saving “Plan”", "Saved “Plan” on this computer", "save “Plan”", ""),
    ("TicketSave", "Saving “Plan”", "Saved “Plan” on this computer", "save “Plan”", ""),
    ("DocumentCreate", "Creating “Plan”", "Created “Plan” on this computer", "create “Plan”", ""),
    ("TicketCreate", "Creating “Plan”", "Created “Plan” on this computer", "create “Plan”", ""),
    ("DocumentMove", "Moving “Plan”", "Moved “Plan”", "move “Plan”", ""),
    ("DocumentPromote", "Approving and merging “Plan”", "Approved and merged “Plan”", "approve and merge “Plan”", ""),
    ("TicketClose", "Closing “Plan”", "Closed “Plan”", "close “Plan”", ""),
    ("ContextPrepare", "Getting a private copy of “Plan” ready", "A private copy of “Plan” is ready", "get a private copy of “Plan” ready", ""),
    ("ContextRelease", "Putting away the private copy of “Plan”", "Put away the private copy of “Plan”", "put away the private copy of “Plan”", ""),
    ("ContextRetire", "Retiring the copy of “Plan”", "Retired the copy of “Plan” on this computer", "retire the copy of “Plan”", ""),
    ("CommentAdd", "Posting a comment on “Plan”", "Posted a comment on “Plan”", "post the comment on “Plan”", ""),
    ("ItemSync", "Sharing “Plan”", "", "share “Plan”", ""),
    ("ItemRepublish", "Republishing “Plan”", "Republished “Plan”", "republish “Plan”", ""),
    ("LocalContextSync", "Repairing “Plan” on this computer", "Repaired “Plan” on this computer", "repair “Plan”", ""),
    ("DocumentRepair", "Repairing a document in demo", "Repaired a document in demo", "repair a document in demo", ""),
    ("TicketRepair", "Repairing a ticket in demo", "Repaired a ticket in demo", "repair a ticket in demo", ""),
];

/// What a share offers in one record state: the button, if any, and the
/// menu, the button's offer first, Dismiss last (B2, B3).
pub(super) type Cell = (Option<&'static str>, &'static [&'static str]);

/// The four record states B3 is read under: Accepted, Finished and Unknown,
/// and Accepted after a Give up.
pub(super) const RECORDS: [(RecordState, bool); 4] = [
    (RecordState::Accepted, false),
    (RecordState::Finished, false),
    (RecordState::Unknown, false),
    (RecordState::Accepted, true),
];

/// One row of B3: the answer, the intent, the extra words the Workspace
/// worked out (R-9), the kind, the title, the description and the cells
/// under each of `RECORDS`.
pub(super) struct ShareRow {
    pub(super) answer: &'static str,
    pub(super) intent: &'static str,
    pub(super) words: Option<&'static str>,
    pub(super) kind: Kind,
    pub(super) title: &'static str,
    pub(super) description: Option<&'static str>,
    pub(super) cells: [Cell; 4],
}

const HISTORY: &str = crate::desktop::model::operations::HISTORY_UNRESOLVED_GUIDANCE;
const NONE: Cell = (None, &["Dismiss"]);
const GIVE_UP: &str = "Stop trying to share";

#[rustfmt::skip]
pub(super) fn share_rows() -> Vec<ShareRow> {
    use Kind::{Failure, Info, Locked, Sharing, Success};
    let row = |answer, intent, words, kind, title, description, cells| ShareRow {
        answer, intent, words, kind, title, description, cells,
    };
    let item = "ItemSync";
    vec![
        row("Published", item, None, Sharing, "Shared “Plan”", None, [NONE; 4]),
        row("Published", "RepoSync", None, Sharing, "Shared trunk", None, [NONE; 4]),
        row("Current", item, None, Success, "“Plan” is the same as the shared copy", None, [NONE; 4]),
        row("Current", "RepoSync", None, Success, "trunk is the same as the shared copy", None, [NONE; 4]),
        row("Stopped", item, None, Info, "Stopped sharing “Plan”",
            Some("Stopped before anything was sent. Your saved work is still on this computer."),
            [(Some("Share"), &["Share", GIVE_UP, "Dismiss"]), (Some("Share"), &["Share", "Dismiss"]),
             (Some("Share"), &["Share", "Dismiss"]), (Some("Share"), &["Share", "Dismiss"])]),
        row("StoppedAfterPart", item, None, Info, "Stopped sharing “Plan”",
            Some("Stopped after part of the work finished. Your saved work is on this computer; Share sends the rest."),
            [(Some("Share"), &["Share", GIVE_UP, "Dismiss"]), (Some("Share"), &["Share", "Dismiss"]),
             (Some("Share"), &["Share", "Dismiss"]), (Some("Share"), &["Share", "Dismiss"])]),
        row("Conflict", item, None, Failure, "You and someone else both changed “Plan”",
            Some("Choose what to keep. Nothing is shared until you do."),
            [(Some("Review changes"), &["Review changes", GIVE_UP, "Dismiss"]), (Some("Review changes"), &["Review changes", "Dismiss"]),
             (Some("Review changes"), &["Review changes", "Dismiss"]), (Some("Review changes"), &["Review changes", "Dismiss"])]),
        row("BranchDeleted", item, None, Failure, "The shared copy of “Plan” was deleted",
            Some("Republish it to share it again."),
            [(None, &["Stop and republish…", GIVE_UP, "Dismiss"]), (Some("Republish…"), &["Republish…", "Dismiss"]),
             (None, &[GIVE_UP, "Dismiss"]), (Some("Republish…"), &["Republish…", "Dismiss"])]),
        row("BranchDeleted", item, Some("Republishing…"), Failure, "The shared copy of “Plan” was deleted",
            Some("Republish it to share it again."),
            [(Some("Republishing…"), &["Republishing…", GIVE_UP, "Dismiss"]), (Some("Republishing…"), &["Republishing…", "Dismiss"]),
             (Some("Republishing…"), &["Republishing…", GIVE_UP, "Dismiss"]), (Some("Republishing…"), &["Republishing…", "Dismiss"])]),
        row("BranchDeleted", "RepoSync", None, Failure, "The shared copy of trunk was deleted",
            Some("Republish it to share it again."),
            [(None, &[GIVE_UP, "Dismiss"]), NONE, NONE, NONE]),
        row("HistoryUnresolved", item, None, Failure, "Couldn't confirm the shared history of “Plan”", Some(HISTORY),
            [(None, &[GIVE_UP, "Dismiss"]), NONE, NONE, NONE]),
        row("NeedsUnlock", item, None, Locked, "Couldn't share “Plan”: your key is locked",
            Some("Unlock it, then share again."),
            [(Some("Unlock…"), &["Unlock…", GIVE_UP, "Dismiss"]), (Some("Share"), &["Share", "Dismiss"]),
             (Some("Unlock…"), &["Unlock…", "Dismiss"]), (Some("Unlock…"), &["Unlock…", "Dismiss"])]),
        row("NeedsKey", item, None, Failure, "Couldn't share “Plan”: choose a key to share with", None,
            [(Some("Open Sharing & keys"), &["Open Sharing & keys", GIVE_UP, "Dismiss"]),
             (Some("Open Sharing & keys"), &["Open Sharing & keys", "Share", "Dismiss"]),
             (Some("Open Sharing & keys"), &["Open Sharing & keys", "Dismiss"]),
             (Some("Open Sharing & keys"), &["Open Sharing & keys", "Dismiss"])]),
        row("NeedsHost", item, None, Failure, "Couldn't share “Plan”: check git.example.invalid before sharing", None,
            [(Some("Check host"), &["Check host", GIVE_UP, "Dismiss"]), (Some("Check host"), &["Check host", "Share", "Dismiss"]),
             (Some("Check host"), &["Check host", "Dismiss"]), (Some("Check host"), &["Check host", "Dismiss"])]),
        row("NeedsHostUnnamed", item, None, Failure, "Couldn't share “Plan”: check the host before sharing", None,
            [(Some("Check host"), &["Check host", GIVE_UP, "Dismiss"]), (Some("Check host"), &["Check host", "Share", "Dismiss"]),
             (Some("Check host"), &["Check host", "Dismiss"]), (Some("Check host"), &["Check host", "Dismiss"])]),
        row("NeedsIdentity", item, None, Failure, "Couldn't share “Plan”: set your name first", None,
            [(Some("Set identity"), &["Set identity", "Retry", GIVE_UP, "Dismiss"]), (Some("Set identity"), &["Set identity", "Share", "Dismiss"]),
             (Some("Set identity"), &["Set identity", "Retry", "Dismiss"]), (Some("Set identity"), &["Set identity", "Retry", "Dismiss"])]),
        row("Busy", item, None, Failure, "Couldn't share “Plan” yet",
            Some("Another share may be holding this repository."),
            [(Some("Show it"), &["Show it", "Retry", GIVE_UP, "Dismiss"]), (Some("Show it"), &["Show it", "Retry", "Dismiss"]),
             (Some("Show it"), &["Show it", "Retry", "Dismiss"]), (Some("Show it"), &["Show it", "Retry", "Dismiss"])]),
        row("Busy", item, Some("May be waiting for Share main"), Failure, "Couldn't share “Plan” yet",
            Some("May be waiting for Share main."),
            [(Some("Show it"), &["Show it", "Retry", GIVE_UP, "Dismiss"]), (Some("Show it"), &["Show it", "Retry", "Dismiss"]),
             (Some("Show it"), &["Show it", "Retry", "Dismiss"]), (Some("Show it"), &["Show it", "Retry", "Dismiss"])]),
        row("NoRemote", item, None, Info, "demo has nowhere to share “Plan” to", None,
            [(Some("Set up sharing"), &["Set up sharing", GIVE_UP, "Dismiss"]), (Some("Set up sharing"), &["Set up sharing", "Dismiss"]),
             (Some("Set up sharing"), &["Set up sharing", "Dismiss"]), (Some("Set up sharing"), &["Set up sharing", "Dismiss"])]),
        row("NoRemote", "RepoSync", None, Info, "demo has nowhere to share trunk to", None,
            [(Some("Set up sharing"), &["Set up sharing", GIVE_UP, "Dismiss"]), (Some("Set up sharing"), &["Set up sharing", "Dismiss"]),
             (Some("Set up sharing"), &["Set up sharing", "Dismiss"]), (Some("Set up sharing"), &["Set up sharing", "Dismiss"])]),
        row("NothingToShare", item, None, Info, "Nothing to share in “Plan” yet", Some("It has no saved changes."),
            [(None, &[GIVE_UP, "Dismiss"]), NONE, NONE, NONE]),
        row("Failed", item, None, Failure, "Couldn't share “Plan”", Some(MESSAGE),
            [(Some("Retry"), &["Retry", GIVE_UP, "Dismiss"]), (Some("Share"), &["Share", "Dismiss"]),
             (Some("Retry"), &["Retry", "Dismiss"]), (Some("Retry"), &["Retry", "Dismiss"])]),
    ]
}

/// Stands for a failed answer's description: the envelope's message, then
/// that the work is safe (B2).
pub(super) const MESSAGE: &str = "<message>";

/// Each intent's subject (A3; R-52, R-57): its name in the fixtures, its
/// family, the kind of item it is about and its own target; a key and an
/// operation stand for the ID the fixture drew.
#[rustfmt::skip]
pub(super) fn subjects() -> Vec<(&'static str, Family, Option<AuthoringKind>, Option<&'static str>)> {
    vec![
        ("RepoCreate", Family::Own(JobKind::RepoCreate), None, None),
        ("RepoEnable", Family::Own(JobKind::RepoEnable), None, None),
        ("RepoRemove", Family::Own(JobKind::RepoRemove), None, None),
        ("RepoIdentitySet", Family::Own(JobKind::RepoIdentitySet), None, None),
        ("RemoteAdd", Family::Own(JobKind::RemoteAdd), None, Some("origin")),
        ("RemoteRemove", Family::Own(JobKind::RemoteRemove), None, Some("origin")),
        ("RemoteSelect", Family::Own(JobKind::RemoteSelect), None, Some("origin")),
        ("RemoteSelectNone", Family::Own(JobKind::RemoteSelect), None, None),
        ("IndexRefresh", Family::Own(JobKind::IndexRefresh), None, None),
        ("IndexRebuild", Family::Own(JobKind::IndexRebuild), None, None),
        ("PollOnce", Family::Check, None, None),
        ("PollConfigure", Family::Own(JobKind::PollConfigure), None, None),
        ("PollPause", Family::Own(JobKind::PollPause), None, None),
        ("PollResume", Family::Own(JobKind::PollResume), None, None),
        ("RepoSync", Family::MainShare, None, None),
        ("FolderCreate", Family::Own(JobKind::FolderCreate), None, Some("docs/folder")),
        ("KeyGenerate", Family::Own(JobKind::KeyGenerate), None, None),
        ("KeyImport", Family::Own(JobKind::KeyImport), None, None),
        ("KeySelect", Family::Own(JobKind::KeySelect), None, Some("<key>")),
        ("KeyClear", Family::Own(JobKind::KeyClear), None, None),
        ("KeyRemove", Family::Own(JobKind::KeyRemove), None, Some("<key>")),
        ("KeyDelete", Family::Own(JobKind::KeyDelete), None, Some("<key>")),
        ("HostApprove", Family::Own(JobKind::HostApprove), None, Some("git.example.invalid:22")),
        ("HostReplace", Family::Own(JobKind::HostReplace), None, Some("git.example.invalid:22")),
        ("OperationResume", Family::Own(JobKind::OperationResume), None, Some("<operation>")),
        ("OperationAbandon", Family::Own(JobKind::OperationAbandon), None, Some("<operation>")),
        ("ConflictResolve", Family::Own(JobKind::ConflictResolve), None, Some("<operation>")),
        ("LocalContextResolve", Family::Own(JobKind::LocalContextResolve), None, Some("<operation>")),
        ("DocumentSave", Family::Save, Some(AuthoringKind::Document), None),
        ("TicketSave", Family::Save, Some(AuthoringKind::Ticket), None),
        ("DocumentCreate", Family::Own(JobKind::DocumentCreate), Some(AuthoringKind::Document), None),
        ("TicketCreate", Family::Own(JobKind::TicketCreate), Some(AuthoringKind::Ticket), None),
        ("DocumentMove", Family::Own(JobKind::DocumentMove), Some(AuthoringKind::Document), None),
        ("DocumentPromote", Family::Own(JobKind::DocumentPromote), Some(AuthoringKind::Document), None),
        ("TicketClose", Family::Own(JobKind::TicketClose), Some(AuthoringKind::Ticket), None),
        ("ContextPrepare", Family::Own(JobKind::ContextPrepare), Some(AuthoringKind::Document), None),
        ("ContextRelease", Family::Own(JobKind::ContextRelease), Some(AuthoringKind::Ticket), None),
        ("ContextRetire", Family::Retire, Some(AuthoringKind::Ticket), None),
        ("CommentAdd", Family::Own(JobKind::Comment), Some(AuthoringKind::Ticket), Some("01ARZ3NDEKTSV4RRFFQ69G5FG1")),
        ("ItemSync", Family::ItemShare, Some(AuthoringKind::Ticket), None),
        ("ItemRepublish", Family::ItemShare, Some(AuthoringKind::Ticket), None),
        ("LocalContextSync", Family::Own(JobKind::LocalContextSync), Some(AuthoringKind::Ticket), None),
        ("DocumentRepair", Family::Own(JobKind::DocumentRepair), None, Some("docs/broken.md")),
        ("TicketRepair", Family::Own(JobKind::TicketRepair), None, Some(".manyhands/tickets/x/ticket.md")),
    ]
}
