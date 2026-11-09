//! The in-flight record and the instance lock (D6 Task 4, R-31, L-6), over
//! real files in a temporary data directory.

use std::{fs, path::Path};

use manyhands::{
    canonical::ItemId,
    repository::{OperationId, RequestId},
};
use tempfile::TempDir;
use time::OffsetDateTime;

use super::{Acquired, ActiveRow, InflightStore, Report, RetirementEnd, RetirementEntry};
use crate::desktop::{
    model::polling::{Answer, CancelCause, NotEligible, PollAttempt, TargetKind, TransportFailure},
    services::worker::RecordState,
};

fn record_path(data: &TempDir) -> std::path::PathBuf {
    data.path().join("inflight").join("record.json")
}

fn on_disk(data: &TempDir) -> serde_json::Value {
    serde_json::from_slice(&fs::read(record_path(data)).unwrap()).unwrap()
}

fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn attempt(root: &str, operation: OperationId, answer: Answer, reported: bool) -> PollAttempt {
    PollAttempt {
        root: root.to_owned(),
        operation,
        answer,
        completed_at: reported.then(OffsetDateTime::now_utc),
        next_eligible_at: None,
        targets: Vec::new(),
        index_pending: false,
    }
}

fn holder(data: &TempDir) -> InflightStore {
    let mut store = InflightStore::open(data.path());
    assert_eq!(store.try_acquire(), Acquired::Now);
    store
}

fn retirement(request: RequestId) -> RetirementEntry {
    RetirementEntry::new(
        request,
        &"/repo".to_owned(),
        TargetKind::Ticket,
        &ItemId::generate(),
    )
}

/// R-31: a poll entry per repository, written before the poll is submitted
/// and replaced by a new ID; removed once the active operation is no longer
/// its ID, or on a terminal answer; a retirement entry removed only when it
/// ended and keeping the first progress event's operation; atomic writes; an
/// unreadable record set aside and reported once; only the lock holder
/// writes, and the lock is tried again (L-6).
#[test]
fn inflight_record() {
    poll_entries_are_one_per_repository_and_written_atomically();
    poll_entries_end_by_the_rules();
    retirement_entries_end_only_when_they_ended();
    an_unreadable_record_is_set_aside_and_reported_once();
    only_the_lock_holder_writes();
}

fn poll_entries_are_one_per_repository_and_written_atomically() {
    let data = tempfile::tempdir().unwrap();
    let mut store = holder(&data);
    let (a, b) = ("/a".to_owned(), "/b".to_owned());
    let (first, second, other) = (OperationId::new(), OperationId::new(), OperationId::new());

    store.poll_submitting(&a, first);
    assert_eq!(on_disk(&data)["polls"].as_array().unwrap().len(), 1);
    // A new ID replaces the repository's entry; another repository's is a
    // second.
    store.poll_submitting(&a, second);
    store.poll_submitting(&b, other);
    let disk = on_disk(&data);
    let polls = disk["polls"].as_array().unwrap();
    assert_eq!(polls.len(), 2, "at most one per repository");
    assert_eq!(disk["version"], 1);
    assert_eq!(store.poll_entry(&a), Some(second));
    assert_eq!(store.poll_entry(&b), Some(other));
    assert!(
        polls
            .iter()
            .all(|entry| entry["operation"] != first.to_string()),
        "the old ID is gone"
    );

    // Written by a temporary file and a rename: nothing is left beside the
    // record and the lock.
    assert_eq!(
        names(&data.path().join("inflight")),
        ["instance.lock", "record.json"]
    );
    // A write that died halfway leaves a temporary file, which neither
    // damages the record nor survives the next open.
    let stray = data
        .path()
        .join("inflight")
        .join(".record.json.0123456789abcdef.tmp");
    fs::write(&stray, b"{\"version\": 1, \"po").unwrap();
    drop(store);
    let again = holder(&data);
    assert!(!stray.exists());
    assert_eq!(again.poll_entry(&a), Some(second), "the record was whole");
    assert!(again.take_report_is_none());
}

/// Rows of the removal rule: (answer, reported by the library, the active
/// operation read after it, whether the entry goes).
fn poll_entries_end_by_the_rules() {
    let entry = OperationId::new();
    let other = OperationId::new();
    let rows: Vec<(&str, Answer, bool, ActiveRow, bool)> = vec![
        (
            "completed",
            Answer::Completed,
            true,
            ActiveRow::Operation(entry),
            true,
        ),
        (
            "a live failure is terminal",
            Answer::Transport(TransportFailure::Unreachable),
            false,
            ActiveRow::Operation(entry),
            true,
        ),
        (
            "a cancel the library reported",
            Answer::Cancelled(CancelCause::Preemption),
            true,
            ActiveRow::Operation(entry),
            true,
        ),
        (
            "a cancel made before the library was called finished nothing",
            Answer::Cancelled(CancelCause::Preemption),
            false,
            ActiveRow::Operation(entry),
            false,
        ),
        (
            "busy, another operation active (L-5)",
            Answer::Busy,
            false,
            ActiveRow::Operation(other),
            true,
        ),
        (
            "busy, the row still the entry's",
            Answer::Busy,
            false,
            ActiveRow::Operation(entry),
            false,
        ),
        (
            "busy, nothing active",
            Answer::Busy,
            false,
            ActiveRow::Nothing,
            true,
        ),
        (
            "busy, the read failed: nothing is dropped for it",
            Answer::Busy,
            false,
            ActiveRow::Unread,
            false,
        ),
        (
            "pending local work, the row still ours",
            Answer::Repository {
                recovery_required: true,
            },
            false,
            ActiveRow::Operation(entry),
            false,
        ),
        (
            "recovery_required, another row active (L-3)",
            Answer::Repository {
                recovery_required: true,
            },
            false,
            ActiveRow::Operation(other),
            true,
        ),
        (
            "a mismatch is not ours whatever is active (L-3)",
            Answer::OperationMismatch,
            false,
            ActiveRow::Operation(entry),
            true,
        ),
        (
            "not eligible, the row still the entry's (L-2)",
            Answer::NotEligible(NotEligible::Paused),
            false,
            ActiveRow::Operation(entry),
            false,
        ),
        (
            "interrupted, the row still the entry's",
            Answer::Interrupted,
            false,
            ActiveRow::Operation(entry),
            false,
        ),
    ];
    for (name, answer, reported, active, goes) in rows {
        let data = tempfile::tempdir().unwrap();
        let mut store = holder(&data);
        store.poll_submitting(&"/a".to_owned(), entry);
        store.poll_answered(&attempt("/a", entry, answer, reported), active);
        assert_eq!(store.poll_entry(&"/a".to_owned()).is_none(), goes, "{name}");
        let polls = on_disk(&data)["polls"].as_array().unwrap().len();
        assert_eq!(polls == 0, goes, "{name}: on disk");
    }

    // An answer of another operation than the entry's ends nothing by being
    // terminal, and an answer for a repository with no entry adds none (L-5).
    let data = tempfile::tempdir().unwrap();
    let mut store = holder(&data);
    store.poll_submitting(&"/a".to_owned(), entry);
    store.poll_answered(
        &attempt("/a", other, Answer::Completed, true),
        ActiveRow::Operation(entry),
    );
    assert_eq!(store.poll_entry(&"/a".to_owned()), Some(entry));
    store.poll_answered(
        &attempt("/b", other, Answer::Busy, false),
        ActiveRow::Operation(other),
    );
    assert_eq!(store.poll_entry(&"/b".to_owned()), None, "never taken over");
    // A submit refused by a closed queue takes its own entry back, not a
    // newer one's.
    store.poll_withdrawn(&"/a".to_owned(), other);
    assert_eq!(store.poll_entry(&"/a".to_owned()), Some(entry));
    store.poll_withdrawn(&"/a".to_owned(), entry);
    assert_eq!(store.poll_entry(&"/a".to_owned()), None);
}

fn retirement_entries_end_only_when_they_ended() {
    let data = tempfile::tempdir().unwrap();
    let mut store = holder(&data);
    let request = RequestId::new();
    let entry = retirement(request);
    store.retirement_submitting(entry.clone());
    assert_eq!(on_disk(&data)["retirements"].as_array().unwrap().len(), 1);
    assert_eq!(store.retirements()[0].operation(), None);

    // The first progress event's operation is the one kept.
    let (first, later) = (OperationId::new(), OperationId::new());
    store.retirement_operation(request, first);
    store.retirement_operation(request, later);
    assert_eq!(store.retirements()[0].operation(), Some(first));
    assert_eq!(store.retirements()[0].request(), Some(request));
    assert_eq!(store.retirements()[0].root(), "/repo");
    assert_eq!(
        store.retirements()[0].item().map(|(kind, _)| kind),
        Some(TargetKind::Ticket)
    );

    // Owed, or not known: kept (Resume needs the item).
    store.retirement_ended(request, RetirementEnd::from(RecordState::Accepted));
    store.retirement_ended(request, RetirementEnd::from(RecordState::Unknown));
    assert_eq!(store.retirements().len(), 1);
    assert_eq!(
        on_disk(&data)["retirements"][0]["operation"],
        first.to_string()
    );

    for end in [
        RetirementEnd::from(RecordState::Finished),
        RetirementEnd::Removed,
        RetirementEnd::NotSent,
    ] {
        store.retirement_submitting(retirement(request));
        store.retirement_ended(request, end);
        assert!(store.retirements().is_empty(), "{end:?}");
        assert!(on_disk(&data)["retirements"].as_array().unwrap().is_empty());
    }

    // It survives a restart of the holder, with its operation.
    store.retirement_submitting(entry);
    store.retirement_operation(request, first);
    drop(store);
    let again = holder(&data);
    assert_eq!(again.retirements().len(), 1);
    assert_eq!(again.retirements()[0].operation(), Some(first));
}

fn an_unreadable_record_is_set_aside_and_reported_once() {
    for (name, bytes) in [
        ("garbage", b"not json at all".to_vec()),
        (
            "a newer version",
            br#"{"version":99,"polls":[],"retirements":[]}"#.to_vec(),
        ),
    ] {
        let data = tempfile::tempdir().unwrap();
        drop(holder(&data));
        fs::write(record_path(&data), &bytes).unwrap();

        let mut store = InflightStore::open(data.path());
        assert_eq!(store.try_acquire(), Acquired::Now, "{name}");
        assert_eq!(store.take_report(), Some(Report::Unreadable), "{name}");
        assert_eq!(store.take_report(), None, "{name}: once");
        // Replaced by an empty record, with the old bytes kept aside.
        assert_eq!(
            on_disk(&data)["polls"].as_array().unwrap().len(),
            0,
            "{name}"
        );
        let aside: Vec<String> = names(&data.path().join("inflight"))
            .into_iter()
            .filter(|file| file.starts_with("record.unreadable-"))
            .collect();
        assert_eq!(aside.len(), 1, "{name}");
        assert_eq!(
            fs::read(data.path().join("inflight").join(&aside[0])).unwrap(),
            bytes,
            "{name}"
        );
        // And it works.
        store.poll_submitting(&"/a".to_owned(), OperationId::new());
        assert_eq!(on_disk(&data)["polls"].as_array().unwrap().len(), 1);
    }
}

fn only_the_lock_holder_writes() {
    let data = tempfile::tempdir().unwrap();
    let (a, b) = ("/a".to_owned(), "/b".to_owned());
    let (held, mine) = (OperationId::new(), OperationId::new());
    let mut first = holder(&data);
    first.poll_submitting(&a, held);
    let before = fs::read(record_path(&data)).unwrap();

    // A second instance: no lock, no write, no takeover, no retirement loop.
    let mut second = InflightStore::open(data.path());
    assert_eq!(second.try_acquire(), Acquired::Elsewhere);
    assert!(!second.holds_lock());
    assert!(!second.runs_retirement_loop());
    assert_eq!(second.takeover_of(&a), None, "it takes nothing over");
    second.poll_submitting(&b, mine);
    assert_eq!(second.poll_entry(&b), Some(mine), "kept in memory");
    assert_eq!(second.takeover_of(&b), None, "not even its own entry");
    second.retirement_submitting(retirement(RequestId::new()));
    assert_eq!(fs::read(record_path(&data)).unwrap(), before, "it wrote");
    assert_eq!(second.poll_entry(&b), Some(mine), "kept in memory");
    assert_eq!(second.try_acquire(), Acquired::Elsewhere);

    // The lock is tried again (window activation): once the first instance
    // is gone it is taken, the previous holder's entries are its to finish
    // (L-1), and what it kept in memory is written.
    assert!(first.holds_lock());
    drop(first);
    assert_eq!(second.try_acquire(), Acquired::Now);
    assert_eq!(second.try_acquire(), Acquired::Already);
    assert!(second.runs_retirement_loop());
    assert_eq!(second.takeover_of(&a), Some(held));
    assert_eq!(second.takeover_of(&b), Some(mine));
    let disk = on_disk(&data);
    assert_eq!(disk["polls"].as_array().unwrap().len(), 2);
    assert_eq!(disk["retirements"].as_array().unwrap().len(), 1);

    // Nowhere to keep a record: never the holder, a data directory that is
    // not one included.
    let mut memory = InflightStore::in_memory();
    assert_eq!(memory.try_acquire(), Acquired::Nowhere);
    let not_a_directory = data.path().join("inflight").join("record.json");
    let mut broken = InflightStore::open(&not_a_directory);
    assert_eq!(broken.try_acquire(), Acquired::Nowhere);
    broken.poll_submitting(&a, held);
    assert_eq!(broken.takeover_of(&a), None);
}

impl InflightStore {
    fn take_report_is_none(&self) -> bool {
        self.report.is_none()
    }
}
