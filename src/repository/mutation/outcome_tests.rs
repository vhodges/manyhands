use super::*;
use crate::results::{CheckpointEffect, DiscoveryEffect, WriteEffect};

/// Every repository error kind and the code a mutation reports for it.
const KINDS: [(RepositoryErrorKind, ResultCode); 35] = [
    (
        RepositoryErrorKind::InvalidSharedKeyMetadata,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::InvalidSharedKeySourcePath,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::SharedKeyRegistryUnavailable,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::SharedKeyMaterialPending,
        ResultCode::InternalError,
    ),
    (RepositoryErrorKind::InvalidPath, ResultCode::InvalidPath),
    (
        RepositoryErrorKind::InaccessibleRepository,
        ResultCode::RepositoryInaccessible,
    ),
    (
        RepositoryErrorKind::NotRepository,
        ResultCode::NotRepository,
    ),
    (
        RepositoryErrorKind::BareRepository,
        ResultCode::BareRepository,
    ),
    (RepositoryErrorKind::DetachedHead, ResultCode::WrongBranch),
    (
        RepositoryErrorKind::WrongCheckedOutBranch,
        ResultCode::WrongBranch,
    ),
    (
        RepositoryErrorKind::DirtyWorktree,
        ResultCode::WorktreeNotClean,
    ),
    (
        RepositoryErrorKind::ConflictedWorktree,
        ResultCode::WorktreeConflicted,
    ),
    (
        RepositoryErrorKind::InvalidConfiguration,
        ResultCode::InvalidConfiguration,
    ),
    (
        RepositoryErrorKind::InvalidPublicationRemote,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::UnavailablePublicationRemote,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::SelectedRemoteRemoval,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::RemoteNameConflict,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::RegistryRefreshPending,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::RepositoryNotRegistered,
        ResultCode::RepositoryNotRegistered,
    ),
    (
        RepositoryErrorKind::IndexUnavailable,
        ResultCode::IndexUnavailable,
    ),
    (RepositoryErrorKind::RepositoryBusy, ResultCode::Busy),
    (
        RepositoryErrorKind::OperationMismatch,
        ResultCode::InternalError,
    ),
    (
        RepositoryErrorKind::RecoveryRequired,
        ResultCode::RecoveryRequired,
    ),
    (
        RepositoryErrorKind::ExternalChange,
        ResultCode::ExternalChange,
    ),
    (
        RepositoryErrorKind::DirtyConfigurationPath,
        ResultCode::WorktreeNotClean,
    ),
    (
        RepositoryErrorKind::InvalidIdentity,
        ResultCode::InvalidInput,
    ),
    (
        RepositoryErrorKind::RepositoryNotEnabled,
        ResultCode::RepositoryNotRegistered,
    ),
    (
        RepositoryErrorKind::MissingAuthoringTarget,
        ResultCode::ItemNotFound,
    ),
    (
        RepositoryErrorKind::OccupiedItemPath,
        ResultCode::OccupiedPath,
    ),
    (
        RepositoryErrorKind::MismatchedAuthoringContext,
        ResultCode::RecoveryRequired,
    ),
    (
        RepositoryErrorKind::RollbackIncomplete,
        ResultCode::RecoveryRequired,
    ),
    (RepositoryErrorKind::Io, ResultCode::InternalError),
    (RepositoryErrorKind::Sqlite, ResultCode::InternalError),
    (RepositoryErrorKind::Git, ResultCode::InternalError),
    (
        RepositoryErrorKind::InjectedFailure,
        ResultCode::InternalError,
    ),
];

#[test]
fn every_repository_error_kind_has_its_code() {
    for (kind, code) in KINDS {
        assert_eq!(repository_error_code(kind), code, "{kind:?}");
        assert!(
            code.failure_class().is_some(),
            "{kind:?}: an error is never reported as a success"
        );
    }
}

fn oid(byte: u8) -> git2::Oid {
    git2::Oid::from_bytes(&[byte; 20]).unwrap()
}

#[test]
fn a_save_outcome_says_what_the_domain_claims_and_what_it_still_owes() {
    let saved = |checkpoint| SaveReport::of_checkpoint(checkpoint, true);
    assert_eq!(
        saved(LocalCheckpoint::Checkpointed { commit_oid: oid(1) }),
        SaveReport::Saved {
            claimed: Some(oid(1)),
            discovered: true
        }
    );
    assert_eq!(
        saved(LocalCheckpoint::NoChange),
        SaveReport::Saved {
            claimed: None,
            discovered: true
        }
    );
    assert_eq!(
        SaveReport::of_checkpoint(
            LocalCheckpoint::RefreshPending { commit_oid: oid(2) },
            false
        ),
        SaveReport::Saved {
            claimed: Some(oid(2)),
            discovered: false
        }
    );
}

#[test]
fn a_save_that_committed_reports_the_commit_and_one_that_did_not_is_a_no_op() {
    let report = SaveReport::Saved {
        claimed: Some(oid(1)),
        discovered: true,
    };
    let (code, effects) = report.result(Some(oid(1)));
    assert_eq!(code, ResultCode::Ok);
    assert_eq!(effects.write, WriteEffect::Written);
    assert_eq!(effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(effects.discovery, DiscoveryEffect::Current);
    assert_eq!(effects.commit_oid, Some(oid(1).to_string()));

    // The domain names a commit this request did not make.
    let (code, effects) = report.result(None);
    assert_eq!(code, ResultCode::AlreadyApplied);
    assert_eq!(effects.write, WriteEffect::Unchanged);
    assert_eq!(effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(effects.discovery, DiscoveryEffect::NotRequested);
    assert_eq!(effects.commit_oid, None);
}

#[test]
fn a_save_whose_hand_off_is_owed_is_discovery_pending() {
    let report = SaveReport::Saved {
        claimed: Some(oid(1)),
        discovered: false,
    };
    let (code, effects) = report.result(Some(oid(1)));
    assert_eq!(code, ResultCode::DiscoveryPending);
    assert_eq!(effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(effects.discovery, DiscoveryEffect::Pending);
    assert!(effects.is_durable());

    let (code, effects) = report.result(None);
    assert_eq!(code, ResultCode::DiscoveryPending);
    assert_eq!(effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(effects.discovery, DiscoveryEffect::Pending);
    assert!(!effects.is_durable());
}

#[test]
fn a_save_that_needs_an_identity_has_done_nothing() {
    let (code, effects) = SaveReport::IdentityRequired.result(None);
    assert_eq!(code, ResultCode::IdentityRequired);
    assert_eq!(effects, Effects::not_requested());
}

#[test]
fn a_stopped_save_reports_what_the_repository_shows_of_it() {
    assert_eq!(stopped_save_effects(None, false), Effects::not_requested());
    let written = stopped_save_effects(None, true);
    assert_eq!(written.write, WriteEffect::Written);
    assert_eq!(written.checkpoint, CheckpointEffect::Pending);
    assert_eq!(written.discovery, DiscoveryEffect::Pending);
    assert_eq!(written.commit_oid, None);
    let committed = stopped_save_effects(Some(oid(3)), true);
    assert_eq!(committed.write, WriteEffect::Written);
    assert_eq!(committed.checkpoint, CheckpointEffect::Committed);
    assert_eq!(committed.discovery, DiscoveryEffect::Pending);
    assert_eq!(committed.commit_oid, Some(oid(3).to_string()));
}

fn actions(code: ResultCode, retry: bool) -> Vec<&'static str> {
    let request_id = RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FX1").unwrap();
    recovery(code, Some("/repository"), request_id, None, retry)
        .iter()
        .map(|action| action.action.as_str())
        .collect()
}

#[test]
fn the_same_request_is_offered_again_only_where_repeating_it_can_help() {
    // A record left accepted is finished by the same request.
    assert_eq!(actions(ResultCode::InternalError, true), ["request.retry"]);
    assert!(actions(ResultCode::InternalError, false).is_empty());
    assert_eq!(actions(ResultCode::Busy, false), ["request.retry"]);
    // Except after an external change to an item: the same request is
    // refused again for as long as the change stands. The caller reads
    // the item again and submits a new request.
    assert!(actions(ResultCode::ExternalChange, true).is_empty());
    assert!(actions(ResultCode::ExternalChange, false).is_empty());
}
