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
fn a_path_nobody_else_changed_is_continued_whatever_the_journal_says() {
    for journal in [JournalRow::Absent, pending(), completed()] {
        assert_eq!(before(journal.clone(), &[]).found(), Found::Continue);
        assert_eq!(
            before(journal, &[(OWN, true)]).found(),
            Found::Continue,
            "the newest change is as intended"
        );
    }
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
    };
    assert_eq!(
        before(owing, &[(FOREIGN, false), (OWN, true)]).found(),
        Found::Foreign {
            own: Some(oid(OWN))
        }
    );
}
