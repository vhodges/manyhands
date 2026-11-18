//! A writer that dies at any step of a write leaves a whole revision: the
//! previous one before the rename, the new one after it.

use crate::support::*;
use manyhands::drafts::{CRASH_EXIT_CODE, DraftId, FailureAction, FailurePoint};

const POINT: &str = "MANYHANDS_DRAFTS_CRASH_POINT";
const DATA: &str = "MANYHANDS_DRAFTS_CRASH_DATA";
const ID: &str = "MANYHANDS_DRAFTS_CRASH_ID";
const TEST: &str = "crash::a_crash_at_any_step_leaves_a_whole_revision";

const POINTS: [(&str, FailurePoint, u64, bool); 5] = [
    // (name, point, revision listed after, temporary file left)
    ("CreateTemp", FailurePoint::CreateTemp, 1, true),
    ("Write", FailurePoint::Write, 1, true),
    ("SyncFile", FailurePoint::SyncFile, 1, true),
    ("Rename", FailurePoint::Rename, 2, false),
    ("SyncFolder", FailurePoint::SyncFolder, 2, false),
];

/// The parent runs one child per point; the child is this test re-run with
/// the point in its environment.
#[test]
fn a_crash_at_any_step_leaves_a_whole_revision() {
    if let Some(point) = std::env::var_os(POINT) {
        crash_child(point.to_str().unwrap());
    }
    for (name, _, revision, left) in POINTS {
        let data = tempfile::tempdir().unwrap();
        let store = store(data.path());
        let id = create(&store).draft().id;
        let output = child(
            TEST,
            &[
                (POINT, name),
                (DATA, data.path().to_str().unwrap()),
                (ID, &id.to_string()),
            ],
        )
        .output()
        .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        ran_one_test(&stdout);
        assert_eq!(
            output.status.code(),
            Some(CRASH_EXIT_CODE),
            "{name}: the child must crash:\n{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let temporary = temporary_files(data.path());
        assert_eq!(temporary.len(), usize::from(left), "{name}: {temporary:?}");
        assert_eq!(listed_revision(&store, id), revision, "{name}");
        assert_eq!(list(&store).len(), 1, "{name}: no temporary file is listed");
        assert!(
            temporary.iter().all(|t| t.starts_with(&format!(".{id}."))),
            "{name}"
        );
        assert_eq!(
            temporary_files(data.path()),
            temporary,
            "{name}: listing removes nothing"
        );

        // The child's exit released its lock; the next claim sweeps.
        let claim = claim(&store, id);
        assert_eq!(claim.draft().revision, revision, "{name}");
        assert!(temporary_files(data.path()).is_empty(), "{name}");
    }
}

fn crash_child(name: &str) -> ! {
    let point = POINTS
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|(_, point, ..)| *point)
        .expect("a known point");
    let data = std::env::var_os(DATA).unwrap();
    let id: DraftId = std::env::var(ID).unwrap().parse().unwrap();
    let store = store(std::path::Path::new(&data));
    let mut claim = claim(&store, id);
    let _ = store.with_failure_point_for_testing(point, FailureAction::Crash);
    let _ = write(&mut claim, 2);
    panic!("the write must have ended the process");
}
