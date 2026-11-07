use git2::Oid;

use super::*;
use crate::repository::recovery::PendingOperation;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).unwrap()
}

const OWN: u8 = 1;
const FOREIGN: u8 = 2;

fn pending() -> JournalRow {
    JournalRow::Pending(PendingOperation::Local {
        state: "created".to_owned(),
        step: None,
    })
}

fn completed() -> JournalRow {
    JournalRow::Final {
        kind: FinalKind::Completed,
        owes_work: false,
        checkpointed: true,
    }
}

/// The evidence of these changes to the path, newest first, each with
/// whether it left the path as intended.
fn before(journal: JournalRow, changes: &[(u8, bool)]) -> Before {
    let changes: Vec<(Oid, bool)> = changes
        .iter()
        .map(|(byte, intended)| (oid(*byte), *intended))
        .collect();
    Before {
        journal,
        evidence: PathEvidence::of_changes_for_testing(&changes),
    }
}

#[test]
fn a_path_nobody_else_changed_is_continued_while_the_request_s_work_is_not_done() {
    for journal in [JournalRow::Absent, pending(), completed()] {
        assert_eq!(before(journal, &[]).found(), Found::Continue);
    }
    // The newest change is as intended, and the operation has not
    // completed: the domain is what finishes it.
    for journal in [JournalRow::Absent, pending()] {
        assert_eq!(before(journal, &[(OWN, true)]).found(), Found::Continue);
    }
}

#[test]
fn a_completed_request_with_its_commit_in_range_is_done() {
    // Nothing remains for the domain to do, and calling it again could
    // only be refused for what has happened to the file since.
    assert_eq!(
        before(completed(), &[(OWN, true)]).found(),
        Found::Done { commit: oid(OWN) }
    );
}

#[test]
fn a_foreign_change_stops_a_request_whose_work_is_not_done() {
    // Nothing of the request's is in range.
    for journal in [JournalRow::Absent, pending(), completed()] {
        assert_eq!(
            before(journal, &[(FOREIGN, false)]).found(),
            Found::Foreign { own: None }
        );
    }
    // The request committed and its operation is still in flight.
    assert_eq!(
        before(pending(), &[(FOREIGN, false), (OWN, true)]).found(),
        Found::Foreign {
            own: Some(oid(OWN))
        }
    );
    // Identical content under a request that never started is not its own.
    assert_eq!(
        before(JournalRow::Absent, &[(FOREIGN, false), (OWN, true)]).found(),
        Found::Foreign { own: None }
    );
    assert!(!before(JournalRow::Absent, &[]).started());
    assert!(before(pending(), &[]).started());
    assert!(before(completed(), &[]).started());
}

#[test]
fn a_completed_request_whose_commit_was_saved_over_is_done() {
    assert_eq!(
        before(completed(), &[(FOREIGN, false), (OWN, true)]).found(),
        Found::Done { commit: oid(OWN) }
    );
    // A row that ended and still owes work is in flight, not done.
    let owing = JournalRow::Final {
        kind: FinalKind::Completed,
        owes_work: true,
        checkpointed: true,
    };
    assert_eq!(
        before(owing, &[(FOREIGN, false), (OWN, true)]).found(),
        Found::Foreign {
            own: Some(oid(OWN))
        }
    );
}

#[test]
fn a_request_whose_row_never_reached_a_checkpoint_has_no_commit_of_its_own() {
    // The row completed with no step of its work recorded: the request
    // stopped for an identity, say. A commit of the intended content in
    // range is someone else's, whatever it was made from.
    let unworked = || JournalRow::Final {
        kind: FinalKind::Completed,
        owes_work: false,
        checkpointed: false,
    };
    let found = before(unworked(), &[(OWN, true)]);
    assert!(found.started());
    assert!(!found.committing());
    assert_eq!(found.found(), Found::Continue);
    assert_eq!(found.reported(&found.evidence), None);
    assert_eq!(
        before(unworked(), &[(FOREIGN, false), (OWN, true)]).found(),
        Found::Foreign { own: None }
    );
    // A pending row may have committed and not recorded it.
    assert!(before(pending(), &[]).committing());
    assert!(!before(JournalRow::Absent, &[]).committing());
}
