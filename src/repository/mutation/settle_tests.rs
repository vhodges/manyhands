use super::*;
use crate::repository::recovery::PendingOperation;

fn stopped(journal: JournalRow) -> Standing {
    Standing::Stopped {
        journal: Some(journal),
    }
}

fn ended(kind: FinalKind, owes_work: bool) -> Standing {
    stopped(JournalRow::Final { kind, owes_work })
}

#[test]
fn a_result_and_owed_work_settle_without_the_journal() {
    assert_eq!(Settlement::of(&Standing::Final), Settlement::Finish);
    assert_eq!(Settlement::of(&Standing::Owed), Settlement::Leave);
}

#[test]
fn a_stopped_request_is_deleted_only_when_nothing_is_in_flight() {
    assert_eq!(
        Settlement::of(&stopped(JournalRow::Absent)),
        Settlement::Delete
    );
    assert_eq!(
        Settlement::of(&ended(FinalKind::Completed, false)),
        Settlement::Delete
    );
    assert_eq!(
        Settlement::of(&ended(FinalKind::Completed, true)),
        Settlement::Leave
    );
    assert_eq!(
        Settlement::of(&stopped(JournalRow::Pending(PendingOperation::Local {
            state: "created".to_owned(),
            step: None,
        }))),
        Settlement::Leave
    );
    // A row that could not be read leaves the record as it is.
    assert_eq!(
        Settlement::of(&Standing::Stopped { journal: None }),
        Settlement::Leave
    );
}

#[test]
fn a_result_the_domain_made_final_is_stored_even_when_work_is_owed() {
    // Asked before `in_flight`: these rows owe work, and asking whether
    // anything is in flight first would leave the record accepted for good.
    for kind in [FinalKind::Cancelled, FinalKind::RetainedForInspection] {
        for owes_work in [false, true] {
            assert_eq!(
                Settlement::of(&ended(kind, owes_work)),
                Settlement::Finish,
                "{kind:?} {owes_work}"
            );
        }
    }
}
