//! Writing a revision: atomic replacement, and what each failure leaves.

use crate::support::*;
use manyhands::drafts::{DraftErrorKind, FailureAction, FailurePoint};

#[test]
fn revisions_are_written_and_read_back_by_a_new_store() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let mut claim = create(&store);
    let id = claim.draft().id;
    assert_eq!(claim.draft().revision, 1);
    assert_eq!(write(&mut claim, 2), Ok(2));
    assert_eq!(write(&mut claim, 3), Ok(3));
    assert_eq!(claim.draft().revision, 3);
    assert!(claim.draft().current == fields(3));
    drop(claim);

    let again = self::store(data.path());
    let claim = self::claim(&again, id);
    assert_eq!(claim.draft().revision, 3);
    assert!(claim.draft().current == fields(3));
    assert!(temporary_files(data.path()).is_empty());
}

#[test]
fn a_written_revision_reports_when_it_was_flushed() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let mut claim = create(&store);
    let written = claim
        .write(2, fields(2), None)
        .expect("the revision is written");
    assert_eq!(written.revision, 2);
    assert!(written.flushed_at == claim.draft().flushed_at);
    assert!(claim.draft().flushed_at >= claim.draft().created_at);
}

#[test]
fn a_lower_revision_is_refused_and_the_stored_one_kept() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let mut claim = create(&store);
    let id = claim.draft().id;
    assert_eq!(write(&mut claim, 3), Ok(3));
    assert_eq!(
        write(&mut claim, 2).map_err(|e| e.kind),
        Err(DraftErrorKind::Io)
    );
    assert_eq!(listed_revision(&store, id), 3);
    assert_eq!(
        write(&mut claim, 3),
        Ok(3),
        "the same revision is rewritten"
    );
}

/// A store with a draft at revision 2, and the failure set on every clone.
fn failing_at(
    data: &std::path::Path,
    point: FailurePoint,
    action: FailureAction,
) -> (manyhands::drafts::DraftStore, manyhands::drafts::DraftClaim) {
    let store = store(data);
    let mut claim = create(&store);
    assert_eq!(write(&mut claim, 2), Ok(2));
    let store = store.with_failure_point_for_testing(point, action);
    (store, claim)
}

#[test]
fn a_failure_before_the_rename_keeps_the_previous_revision_and_no_temporary_file() {
    let cases = [
        (FailurePoint::CreateTemp, DraftErrorKind::Io),
        (FailurePoint::Write, DraftErrorKind::Io),
        (FailurePoint::Write, DraftErrorKind::NoSpace),
        (FailurePoint::SyncFile, DraftErrorKind::Io),
        (FailurePoint::Rename, DraftErrorKind::Io),
    ];
    for (point, kind) in cases {
        let data = tempfile::tempdir().unwrap();
        let (store, mut claim) = failing_at(data.path(), point, FailureAction::Fail(kind));
        let id = claim.draft().id;
        assert_eq!(
            write(&mut claim, 3).map_err(|e| e.kind),
            Err(kind),
            "{point:?}"
        );
        assert_eq!(listed_revision(&store, id), 2, "{point:?}");
        assert_eq!(claim.draft().revision, 2, "{point:?}");
        assert!(temporary_files(data.path()).is_empty(), "{point:?}");
    }
}

#[test]
fn a_failed_folder_sync_reports_the_failure_with_the_new_revision_in_place() {
    let data = tempfile::tempdir().unwrap();
    let (store, mut claim) = failing_at(
        data.path(),
        FailurePoint::SyncFolder,
        FailureAction::Fail(DraftErrorKind::Io),
    );
    let id = claim.draft().id;
    assert_eq!(
        write(&mut claim, 3).map_err(|e| e.kind),
        Err(DraftErrorKind::Io)
    );
    assert_eq!(listed_revision(&store, id), 3);
    assert_eq!(claim.draft().revision, 3, "the claim knows what is stored");
    assert!(temporary_files(data.path()).is_empty());
}

#[test]
fn a_failure_point_applies_to_every_clone_of_the_store() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let mut claim = create(&store);
    let _ = store.clone().with_failure_point_for_testing(
        FailurePoint::SyncFile,
        FailureAction::Fail(DraftErrorKind::Io),
    );
    assert_eq!(
        write(&mut claim, 2).map_err(|e| e.kind),
        Err(DraftErrorKind::Io)
    );
}

#[test]
fn a_create_whose_first_write_fails_after_the_rename_leaves_no_draft() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path()).with_failure_point_for_testing(
        FailurePoint::SyncFolder,
        FailureAction::Fail(DraftErrorKind::Io),
    );
    let created = store.create(target(), None, fields(1)).map(|_| ());
    assert_eq!(created.map_err(|e| e.kind), Err(DraftErrorKind::Io));
    assert!(list(&store).is_empty());
    assert!(names(data.path()).is_empty(), "{:?}", names(data.path()));
}
