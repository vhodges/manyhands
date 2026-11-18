//! P6: the words of the state rows. The tables the rows read, row by row:
//! the dispositions, the pending actions and next actions, the counts and
//! `repository_detail`. The rows' own titles and descriptions are checked
//! with their keys, buttons and groups in `states`.

use manyhands::repository::OperationAction;

use crate::desktop::model::{
    activity::copy::{
        AN_OPERATION, action_words, brought_description, brought_title, disposition_name,
        disposition_sentence, next_action_sentence, recovered_title, repository_detail,
    },
    polling::Disposition,
};
use manyhands::repository::OperationNextAction;

/// P6: B4's tables, row by row; the pending action table is total.
#[test]
fn state_copy_table() {
    // The five dispositions' sentences, S5's table: every problem has one.
    let sentences = [
        (
            Disposition::Dirty,
            "Dirty",
            "It has changes on this computer that aren't saved yet.",
        ),
        (
            Disposition::Conflicted,
            "Conflicted",
            "It has a conflict to resolve.",
        ),
        (
            Disposition::Inaccessible,
            "Inaccessible",
            "Manyhands can't read it.",
        ),
        (
            Disposition::Nonconforming,
            "Nonconforming",
            "It needs fixing before it can be shared.",
        ),
        (
            Disposition::BlockedByLocalState,
            "Blocked by local state",
            "Something on this computer is in the way of updating it.",
        ),
        (
            Disposition::MaterializationIncomplete,
            "Materialization incomplete",
            "Bringing it in didn't finish.",
        ),
        (
            Disposition::Malformed,
            "Malformed",
            "Its file can't be read as a ticket or document.",
        ),
        (
            Disposition::RemotelyDeleted,
            "Remotely deleted",
            "Its shared copy was deleted.",
        ),
        (
            Disposition::HistoryUnknown,
            "History unknown",
            "Manyhands can't confirm its shared history.",
        ),
        (
            Disposition::NotReached,
            "Not reached",
            "The check didn't reach it this time.",
        ),
    ];
    for (disposition, name, sentence) in sentences {
        assert_eq!(
            disposition_sentence(disposition),
            Some(sentence),
            "{disposition:?}"
        );
        assert_eq!(disposition_name(disposition), name, "{disposition:?}");
    }
    // The items not shared yet are named too, and the rest have no sentence:
    // they are not problems.
    for (disposition, name) in [
        (Disposition::LocalAhead, "Local ahead"),
        (Disposition::Unpublished, "Unpublished"),
        (Disposition::Diverged, "Diverged"),
    ] {
        assert_eq!(disposition_name(disposition), name, "{disposition:?}");
        assert_eq!(disposition_sentence(disposition), None, "{disposition:?}");
    }
    for disposition in [
        Disposition::Current,
        Disposition::FastForwarded,
        Disposition::Materialized,
        Disposition::Closed,
        Disposition::Promoted,
    ] {
        assert_eq!(disposition_sentence(disposition), None, "{disposition:?}");
    }

    // The pending action table is total: every action the library lists has
    // its words, and "An operation" is only for what has none.
    let words = [
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
        (OperationAction::SynchronizePrimary, "Sharing trunk"),
        (OperationAction::Promote, "Approving and merging"),
        (OperationAction::Close, "Closing"),
        (OperationAction::GenerateKey, "Creating a key"),
        (OperationAction::DeleteKey, "Deleting a key"),
    ];
    assert_eq!(words.len(), OperationAction::ALL.len());
    for action in OperationAction::ALL {
        let entry = words
            .iter()
            .find(|(listed, _)| *listed == action)
            .unwrap_or_else(|| panic!("{action:?} has no row in the test table"));
        assert_eq!(action_words(action, "trunk"), entry.1, "{action:?}");
        assert_ne!(action_words(action, "trunk"), AN_OPERATION, "{action:?}");
    }
    assert_eq!(AN_OPERATION, "An operation");

    // The next actions' sentences, and "Nothing to do." for none.
    for (next, sentence) in [
        (Some(OperationNextAction::Resume), "Resume finishes it."),
        (
            Some(OperationNextAction::RetryGeneration),
            "Resume tries again.",
        ),
        (
            Some(OperationNextAction::ReviewDeletionAgain),
            "Review it again.",
        ),
        (
            Some(OperationNextAction::InspectRetainedFiles),
            "Some files were kept for you to look at.",
        ),
        (None, "Nothing to do."),
    ] {
        assert_eq!(next_action_sentence(next), sentence, "{next:?}");
    }

    // Counts have explicit singular and plural forms (S2, S13).
    assert_eq!(brought_description(1), "Brought in 1 change.");
    assert_eq!(brought_description(4), "Brought in 4 changes.");
    assert_eq!(brought_title(1, "demo"), "Brought in 1 change to demo");
    assert_eq!(brought_title(4, "demo"), "Brought in 4 changes to demo");
    assert_eq!(
        recovered_title(1),
        "1 draft recovered from an earlier session"
    );
    assert_eq!(
        recovered_title(2),
        "2 drafts recovered from an earlier session"
    );

    // The repository's detail starts at its main branch and says the shared
    // copy by name only, never the folder or the location (R-48).
    for (primary, remote, lines) in [
        (
            Some("trunk"),
            Some("origin"),
            vec!["Main branch: trunk", "Shared copy: origin"],
        ),
        (Some("trunk"), None, vec!["Main branch: trunk"]),
        (None, Some("origin"), vec!["Shared copy: origin"]),
        (None, None, vec![]),
    ] {
        assert_eq!(
            repository_detail(primary, remote),
            lines,
            "{primary:?} {remote:?}"
        );
    }
}
