use git2::{Repository, Signature, Time};
use serde_json::json;

use super::*;

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const MAIN: &str = "refs/heads/main";
const OTHER: &str = "refs/heads/other";
const TICKET: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";

/// A commit with `parents` and a tree holding `name`, on no reference.
fn commit(repository: &Repository, parents: &[Oid], name: &str) -> Oid {
    let signature = Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).unwrap();
    let blob = repository.blob(name.as_bytes()).unwrap();
    let mut builder = repository.treebuilder(None).unwrap();
    builder.insert(name, blob, 0o100644).unwrap();
    let tree = repository.find_tree(builder.write().unwrap()).unwrap();
    let parents: Vec<_> = parents
        .iter()
        .map(|parent| repository.find_commit(*parent).unwrap())
        .collect();
    let parents: Vec<_> = parents.iter().collect();
    repository
        .commit(None, &signature, &signature, name, &tree, &parents)
        .unwrap()
}

fn point(repository: &Repository, reference: &str, oid: Oid) {
    repository.reference(reference, oid, true, "test").unwrap();
}

/// Where the ticket's branch, primary and HEAD stand when a finished request
/// is replayed, and the commit its result named: `closed`, on the ticket's
/// branch when it was made.
enum Case {
    /// The branch stands at the commit, primary behind it.
    AtTheTip,
    /// The branch has moved on past the commit.
    PastIt,
    /// The branch was reset behind the commit.
    ResetBehind,
    /// The branch is gone, and primary holds the commit by a merge.
    GoneMerged,
    /// The branch is gone and primary holds a squash of it, while HEAD is on
    /// another branch that holds the commit.
    GoneSquashedHeadElsewhere,
}

#[test]
fn a_result_s_commit_is_held_where_its_request_left_it() {
    let rows = [
        (
            "a close's commit at its branch's tip, primary behind",
            Case::AtTheTip,
            Some(Command::TicketClose),
            "pull_request_closure",
            true,
        ),
        (
            "a withdraw's commit at its branch's tip, primary behind",
            Case::AtTheTip,
            Some(Command::TicketWithdrawClosure),
            "pull_request_closure",
            true,
        ),
        (
            "the branch moved past the commit",
            Case::PastIt,
            Some(Command::TicketClose),
            "pull_request_closure",
            true,
        ),
        (
            "the branch reset behind the commit",
            Case::ResetBehind,
            Some(Command::TicketClose),
            "pull_request_closure",
            false,
        ),
        (
            "the branch gone, primary holds the commit by a merge",
            Case::GoneMerged,
            Some(Command::TicketClose),
            "pull_request_closure",
            true,
        ),
        (
            // Never whatever is checked out (Known Limit: a squash or a
            // rebase merge replays no more).
            "the branch gone, a squash, HEAD on a branch that holds the commit",
            Case::GoneSquashedHeadElsewhere,
            Some(Command::TicketClose),
            "pull_request_closure",
            false,
        ),
        (
            "a direct close's data, the commit only on the ticket's branch",
            Case::AtTheTip,
            Some(Command::TicketClose),
            "lifecycle",
            false,
        ),
        (
            "another command's data tagged as a branch record's",
            Case::AtTheTip,
            Some(Command::TicketSave),
            "pull_request_closure",
            false,
        ),
    ];
    for (what, case, command, tag, held) in rows {
        let directory = tempfile::tempdir().unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(directory.path(), &options).unwrap();
        let base = commit(&repository, &[], "base");
        point(&repository, MAIN, base);
        let closed = commit(&repository, &[base], "closed");
        point(&repository, TICKET, closed);
        match case {
            Case::AtTheTip => {}
            Case::PastIt => {
                let later = commit(&repository, &[closed], "later");
                point(&repository, TICKET, later);
            }
            Case::ResetBehind => point(&repository, TICKET, base),
            Case::GoneMerged => {
                repository.find_reference(TICKET).unwrap().delete().unwrap();
                let merge = commit(&repository, &[base, closed], "merge");
                point(&repository, MAIN, merge);
            }
            Case::GoneSquashedHeadElsewhere => {
                repository.find_reference(TICKET).unwrap().delete().unwrap();
                let squash = commit(&repository, &[base], "squash");
                point(&repository, MAIN, squash);
                point(&repository, OTHER, closed);
                repository.set_head(OTHER).unwrap();
            }
        }
        let data = json!({"type": tag});
        assert_eq!(
            result_commit_held(
                directory.path(),
                command,
                Some(&data),
                &format!("ticket/{ITEM}"),
                Some(MAIN),
                closed,
            ),
            held,
            "{what}"
        );
    }
}
