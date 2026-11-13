//! R-26: B2's local external rule also sends the whole set outside when
//! the inspection's way on is `external_repair`, or when it cannot start a
//! resolution while the read is neither historical nor resumable. Real
//! local conflicts where a fixture reaches the shape; the inspection is
//! canned, as the parent module's, where none does.

use manyhands::repository::{
    FailurePoint, LocalContextSyncEligibility, LocalContextSyncNextAction, LocalContextSyncReason,
    RequestId,
};

use super::{TICKET, canonical, content, entry, external, inspection};
use crate::desktop::{
    model::conflict::{ConflictModel, ExternalReason},
    test_support::fixtures::{Fixture, ticket_path},
};

/// Real: an untracked file in the work context, or its conflicted file
/// edited outside, makes the library's way on `external_repair` (as
/// `tests/mutation_replay/local_sync_inspection_spec.rs`
/// `spec_observed_resolution_path_cannot_return_to_its_preimage` reads
/// it), and the whole set goes outside.
#[test]
fn an_external_repair_sends_the_local_set_outside() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let worktree = fixture.context_worktree(&root, TICKET);

    std::fs::write(worktree.join("untracked.txt"), b"x").unwrap();
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    assert_eq!(
        inspection.next_action,
        LocalContextSyncNextAction::ExternalRepair
    );
    assert_eq!(
        inspection.eligibility,
        Some(LocalContextSyncEligibility::EligibleCanonical)
    );
    let model = ConflictModel::from_local(&inspection, &contents);
    assert_eq!(
        external(&model),
        [(ticket_path(TICKET), ExternalReason::Unsupported)]
    );
    assert!(super::super::local_set_is_external(&inspection), "not read");

    std::fs::remove_file(worktree.join("untracked.txt")).unwrap();
    let mine = contents[0].local.clone().unwrap();
    std::fs::write(worktree.join(ticket_path(TICKET)), mine).unwrap();
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    let model = ConflictModel::from_local(&inspection, &contents);
    assert_eq!(
        external(&model),
        [(ticket_path(TICKET), ExternalReason::Unsupported)]
    );
}

/// Canned, the inspection of the parent module's shape with
/// `can_start_resolution: false` and the way on still `resolve`: the
/// library answers false when its image or head moved
/// (`src/repository/local_sync/conflict.rs` `inspect`). A read that is
/// neither historical nor resumable goes outside.
#[test]
fn a_local_set_that_cannot_start_goes_outside() {
    let ticket_entry = entry(&ticket_path(TICKET), true);
    let mut cannot = inspection(
        LocalContextSyncReason::CanonicalConflict,
        LocalContextSyncEligibility::EligibleCanonical,
        vec![ticket_entry.clone()],
    );
    cannot.can_start_resolution = false;
    let model = ConflictModel::from_local(&cannot, &[content(&ticket_entry.token, false)]);
    assert_eq!(
        external(&model),
        [(ticket_path(TICKET), ExternalReason::Unsupported)]
    );
}

/// R-26's exceptions, real: a historical read (an attempt has written
/// paths, P24 and K-10; `LocalSyncAfterResolutionPath`) and a resumable one
/// (its commit prepared, P15; `LocalSyncAfterCandidate`, as
/// `tests/mutation_replay/local_sync_spec.rs`
/// `spec_resume_aliases_use_canonical_local_scope` stops one) both answer
/// `can_start_resolution: false`, and are not sent outside. The real
/// resumable read is historical too (M-g): no fixture reaches a resumable
/// read that is not, so none is canned.
#[test]
fn a_historical_or_resumable_local_read_is_not_sent_outside() {
    let fixture = Fixture::new();
    for (name, failure) in [
        ("a", FailurePoint::LocalSyncAfterResolutionPath),
        ("b", FailurePoint::LocalSyncAfterCandidate),
    ] {
        let root = fixture.create_repository(name);
        let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
        fixture.stop_local_resolution(&root, operation, RequestId::new(), failure);
        let (inspection, contents) = fixture.read_local_conflict(&root, operation);
        assert!(!inspection.can_start_resolution);
        let model = ConflictModel::from_local(&inspection, &contents);
        assert!(model.historical());
        assert_eq!(
            model.resume(),
            failure == FailurePoint::LocalSyncAfterCandidate,
            "the prepared commit is resumed"
        );
        assert_eq!(canonical(&model).len(), 1, "not sent outside");
    }
}

/// R-26 as refined (review I-B), real: with no Git identity the library
/// answers `can_start_resolution: false` and the way on `set_identity`
/// (`src/repository/local_sync/request.rs`, the `identity_required`
/// reason). The set stays on the sheet, where the resolution's
/// `identity_required` answer points to Set identity.
#[test]
fn a_missing_identity_keeps_the_local_set_on_the_sheet() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let git = git2::Repository::open(&root).unwrap();
    let mut config = git
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    assert_eq!(
        inspection.next_action,
        LocalContextSyncNextAction::SetIdentity
    );
    assert!(!inspection.can_start_resolution);
    let model = ConflictModel::from_local(&inspection, &contents);
    assert_eq!(canonical(&model).len(), 1);
}
