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

#[test]
fn a_commit_the_evidence_check_could_not_confirm_leaves_the_record_accepted() {
    // The domain claims a commit and Git could not be read to confirm it:
    // finishing the record would store a no-op for a save that committed.
    assert_eq!(Settlement::of(&Standing::Unconfirmed), Settlement::Leave);
}

#[test]
fn a_re_entry_that_stopped_before_its_domain_call_keeps_what_an_earlier_attempt_left() {
    // No attempt ever reached the domain: nothing is in flight and the
    // request ID is free again.
    assert_eq!(
        Settlement::of(&Standing::NotRun {
            journal: JournalRow::Absent
        }),
        Settlement::Delete
    );
    // An earlier attempt started. Whether its row is pending or has
    // completed, the request's result is still owed, and this call
    // learned nothing of it.
    for journal in [
        JournalRow::Pending(PendingOperation::Local {
            state: "created".to_owned(),
            step: None,
        }),
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
        },
    ] {
        assert_eq!(
            Settlement::of(&Standing::NotRun { journal }),
            Settlement::Leave
        );
    }
}
