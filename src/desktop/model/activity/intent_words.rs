//! B2's table: what each intent is called while it is queued, running and
//! done, from its Doing, Done and Verb, so no template repeats the subject.
//! A row names a repository by its folder name, an item by its title as read
//! (else its short code), a remote by its name and a host by its host name;
//! never a path, a draft's own title or an address (decision 21).

use manyhands::{canonical::ItemId, repository::AuthoringKind};

use crate::desktop::services::requests::Intent;

/// What a row names, as B1 words it.
pub(super) struct Names {
    /// The repository: its folder name, or "Application" for a key command.
    pub repo: String,
    /// The repository's main branch when known, else "main".
    pub primary: String,
    /// The item as `copy::item_name` words it, for an item intent.
    pub item: Option<String>,
}

/// An intent's words in B2's table.
pub(super) struct Words {
    /// "{Doing}": a running request reads "{Doing}…".
    pub doing: String,
    /// "{Done}"; `None` where B3 words a share's finish.
    pub done: Option<String>,
    /// "{verb}": "Waiting to {verb}", "Couldn't {verb}", "Cancelled: {verb}".
    pub verb: String,
    /// The Done row's own description.
    pub note: Option<&'static str>,
}

/// The item an intent is about, by its kind.
pub(super) fn item_of(intent: &Intent) -> Option<(AuthoringKind, &ItemId)> {
    use AuthoringKind::{Document, Ticket};
    match intent {
        Intent::DocumentSave { id, .. }
        | Intent::DocumentCreate { id, .. }
        | Intent::DocumentMove { id, .. }
        | Intent::DocumentPromote { id, .. } => Some((Document, id)),
        Intent::TicketSave { id, .. }
        | Intent::TicketCreate { id, .. }
        | Intent::TicketClose { id, .. } => Some((Ticket, id)),
        Intent::ContextPrepare { kind, id, .. }
        | Intent::ContextRelease { kind, id, .. }
        | Intent::ContextRetire { kind, id, .. }
        | Intent::CommentAdd { kind, id, .. }
        | Intent::ItemSync { kind, id, .. }
        | Intent::ItemRepublish { kind, id, .. }
        | Intent::LocalContextSync { kind, id, .. } => Some((*kind, id)),
        _ => None,
    }
}

fn words(doing: String, done: String, verb: String) -> Words {
    Words {
        doing,
        done: Some(done),
        verb,
        note: None,
    }
}

/// The words of `intent`, over what its row names.
pub(super) fn intent_words(intent: &Intent, names: &Names) -> Words {
    let repo = &names.repo;
    let primary = &names.primary;
    let item = names.item.as_deref().unwrap_or("it");
    let both = |doing: &str, done: &str, verb: &str| {
        words(doing.to_owned(), done.to_owned(), verb.to_owned())
    };
    match intent {
        Intent::RepoCreate { .. } => both(
            &format!("Creating {repo}"),
            &format!("Created {repo}"),
            &format!("create {repo}"),
        ),
        Intent::RepoEnable { .. } => both(
            &format!("Adding {repo}"),
            &format!("Added {repo} to Manyhands"),
            &format!("add {repo}"),
        ),
        Intent::RepoRemove { .. } => Words {
            note: Some("Its files are unchanged."),
            ..both(
                &format!("Removing {repo} from Manyhands"),
                &format!("Removed {repo} from Manyhands"),
                &format!("remove {repo} from Manyhands"),
            )
        },
        Intent::RepoIdentitySet { .. } => both(
            &format!("Setting your name for {repo}"),
            &format!("Set your name for {repo}"),
            &format!("set your name for {repo}"),
        ),
        Intent::RemoteAdd { name, .. } => both(
            &format!("Adding the shared copy {name} to {repo}"),
            &format!("Added the shared copy {name} to {repo}"),
            &format!("add the shared copy {name}"),
        ),
        Intent::RemoteRemove { name, .. } => both(
            &format!("Removing the shared copy {name}"),
            &format!("Removed the shared copy {name} from {repo}"),
            &format!("remove the shared copy {name}"),
        ),
        Intent::RemoteSelect { name, .. } => both(
            &format!("Choosing how {repo} shares"),
            &match name {
                Some(name) => format!("{repo} now shares through {name}"),
                None => format!("{repo} no longer shares"),
            },
            &format!("choose how {repo} shares"),
        ),
        Intent::IndexRefresh { .. } => both(
            &format!("Refreshing {repo}"),
            &format!("Refreshed {repo}"),
            &format!("refresh {repo}"),
        ),
        Intent::IndexRebuild { .. } => both(
            &format!("Rebuilding the search index for {repo}"),
            &format!("Rebuilt the search index for {repo}"),
            &format!("rebuild the search index for {repo}"),
        ),
        Intent::PollOnce { .. } => both(
            &format!("Checking {repo} for updates"),
            &format!("Checked {repo} for updates"),
            &format!("check {repo} for updates"),
        ),
        Intent::PollConfigure { .. } => both(
            &format!("Changing how often {repo} is checked"),
            &format!("Changed how often {repo} is checked"),
            &format!("change how often {repo} is checked"),
        ),
        Intent::PollPause { .. } => both(
            &format!("Pausing checks of {repo}"),
            &format!("Paused checking {repo} for updates"),
            &format!("pause checking {repo}"),
        ),
        Intent::PollResume { .. } => both(
            &format!("Resuming checks of {repo}"),
            &format!("Resumed checking {repo} for updates"),
            &format!("resume checking {repo}"),
        ),
        Intent::RepoSync { .. } => Words {
            doing: format!("Sharing {primary}"),
            done: None,
            verb: format!("share {primary}"),
            note: None,
        },
        Intent::FolderCreate { .. } => both(
            &format!("Creating a folder in {repo}"),
            &format!("Created a folder in {repo}"),
            &format!("create a folder in {repo}"),
        ),
        Intent::KeyGenerate { .. } => both(
            "Creating a key",
            "Created a key for sharing",
            "create a key",
        ),
        Intent::KeyImport { .. } => both("Adding a key", "Added a key for sharing", "add the key"),
        Intent::KeySelect { .. } => both(
            "Choosing the key for sharing",
            "Chose the key used for sharing",
            "choose the key",
        ),
        Intent::KeyClear => both(
            "Clearing the key choice",
            "No key is chosen for sharing now",
            "clear the key choice",
        ),
        Intent::KeyRemove { .. } => Words {
            note: Some("Its files are unchanged."),
            ..both(
                "Removing a key from Manyhands",
                "Removed a key from Manyhands",
                "remove the key from Manyhands",
            )
        },
        Intent::KeyDelete { .. } => both("Deleting a key", "Deleted a key", "delete the key"),
        Intent::HostApprove { authority, .. } => both(
            &format!("Trusting {}", authority.host),
            &format!("Trusted {} for sharing", authority.host),
            &format!("trust {}", authority.host),
        ),
        Intent::HostReplace { authority, .. } => both(
            &format!("Updating the trusted key for {}", authority.host),
            &format!("Updated the trusted key for {}", authority.host),
            &format!("update the trusted key for {}", authority.host),
        ),
        Intent::OperationResume { .. } => both(
            &format!("Finishing an operation in {repo}"),
            &format!("Finished an operation in {repo}"),
            &format!("finish the operation in {repo}"),
        ),
        Intent::OperationAbandon { .. } => both(
            &format!("Abandoning an operation in {repo}"),
            &format!("Abandoned an operation in {repo}"),
            &format!("abandon the operation in {repo}"),
        ),
        Intent::ConflictResolve { .. } => both(
            &format!("Saving your choices in {repo}"),
            &format!("Resolved a conflict in {repo}"),
            &format!("resolve the conflict in {repo}"),
        ),
        Intent::LocalContextResolve { .. } => both(
            &format!("Saving your choices in {repo}"),
            &format!("Resolved a conflict on this computer in {repo}"),
            &format!("resolve the conflict in {repo}"),
        ),
        Intent::DocumentSave { .. } | Intent::TicketSave { .. } => both(
            &format!("Saving {item}"),
            &format!("Saved {item} on this computer"),
            &format!("save {item}"),
        ),
        Intent::DocumentCreate { .. } | Intent::TicketCreate { .. } => both(
            &format!("Creating {item}"),
            &format!("Created {item} on this computer"),
            &format!("create {item}"),
        ),
        Intent::DocumentMove { .. } => both(
            &format!("Moving {item}"),
            &format!("Moved {item}"),
            &format!("move {item}"),
        ),
        Intent::DocumentPromote { .. } => both(
            &format!("Approving and merging {item}"),
            &format!("Approved and merged {item}"),
            &format!("approve and merge {item}"),
        ),
        Intent::TicketClose { .. } => both(
            &format!("Closing {item}"),
            &format!("Closed {item}"),
            &format!("close {item}"),
        ),
        Intent::ContextPrepare { .. } => both(
            &format!("Getting a private copy of {item} ready"),
            &format!("A private copy of {item} is ready"),
            &format!("get a private copy of {item} ready"),
        ),
        Intent::ContextRelease { .. } => both(
            &format!("Putting away the private copy of {item}"),
            &format!("Put away the private copy of {item}"),
            &format!("put away the private copy of {item}"),
        ),
        Intent::ContextRetire { .. } => both(
            &format!("Retiring the copy of {item}"),
            &format!("Retired the copy of {item} on this computer"),
            &format!("retire the copy of {item}"),
        ),
        Intent::CommentAdd { .. } => both(
            &format!("Posting a comment on {item}"),
            &format!("Posted a comment on {item}"),
            &format!("post the comment on {item}"),
        ),
        Intent::ItemSync { .. } => Words {
            doing: format!("Sharing {item}"),
            done: None,
            verb: format!("share {item}"),
            note: None,
        },
        Intent::ItemRepublish { .. } => both(
            &format!("Republishing {item}"),
            &format!("Republished {item}"),
            &format!("republish {item}"),
        ),
        Intent::LocalContextSync { .. } => both(
            &format!("Repairing {item} on this computer"),
            &format!("Repaired {item} on this computer"),
            &format!("repair {item}"),
        ),
        Intent::DocumentRepair { .. } => both(
            &format!("Repairing a document in {repo}"),
            &format!("Repaired a document in {repo}"),
            &format!("repair a document in {repo}"),
        ),
        Intent::TicketRepair { .. } => both(
            &format!("Repairing a ticket in {repo}"),
            &format!("Repaired a ticket in {repo}"),
            &format!("repair a ticket in {repo}"),
        ),
    }
}
