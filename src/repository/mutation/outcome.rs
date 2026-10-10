//! Domain outcomes and errors as the codes and effects of a result.
//!
//! Every mapping here is a `match` with no wildcard arm, so a variant added
//! later does not compile until it is given a code. None formats the error
//! it maps.

use crate::results::{CheckpointEffect, DiscoveryEffect, Effects, ResultCode, WriteEffect};

use super::super::{LocalCheckpoint, RepositoryErrorKind, SaveOutcome};

/// The code a mutation reports for a domain error of `kind`.
///
/// The kinds a ticket create or save can return have their codes. The
/// kinds only another command returns are `internal_error` here, each in
/// its own group, until the task that binds that command gives them
/// theirs.
pub(crate) fn repository_error_code(kind: RepositoryErrorKind) -> ResultCode {
    match kind {
        RepositoryErrorKind::InvalidPath => ResultCode::InvalidPath,
        RepositoryErrorKind::InaccessibleRepository => ResultCode::RepositoryInaccessible,
        RepositoryErrorKind::NotRepository => ResultCode::NotRepository,
        RepositoryErrorKind::BareRepository => ResultCode::BareRepository,
        RepositoryErrorKind::DetachedHead | RepositoryErrorKind::WrongCheckedOutBranch => {
            ResultCode::WrongBranch
        }
        RepositoryErrorKind::DirtyWorktree | RepositoryErrorKind::DirtyConfigurationPath => {
            ResultCode::WorktreeNotClean
        }
        RepositoryErrorKind::ConflictedWorktree => ResultCode::WorktreeConflicted,
        RepositoryErrorKind::InvalidConfiguration => ResultCode::InvalidConfiguration,
        RepositoryErrorKind::RepositoryNotRegistered
        | RepositoryErrorKind::RepositoryNotEnabled => ResultCode::RepositoryNotRegistered,
        RepositoryErrorKind::IndexUnavailable => ResultCode::IndexUnavailable,
        RepositoryErrorKind::RepositoryBusy => ResultCode::Busy,
        RepositoryErrorKind::RecoveryRequired
        | RepositoryErrorKind::RollbackIncomplete
        | RepositoryErrorKind::MismatchedAuthoringContext => ResultCode::RecoveryRequired,
        RepositoryErrorKind::ExternalChange => ResultCode::ExternalChange,
        RepositoryErrorKind::InvalidIdentity => ResultCode::InvalidInput,
        RepositoryErrorKind::MissingAuthoringTarget => ResultCode::ItemNotFound,
        RepositoryErrorKind::OccupiedItemPath => ResultCode::OccupiedPath,
        // The boundary allocates operation IDs, so a mismatch is its own
        // fault.
        RepositoryErrorKind::OperationMismatch => ResultCode::InternalError,
        // A backend failure. Its text stays in the error.
        RepositoryErrorKind::Io
        | RepositoryErrorKind::Sqlite
        | RepositoryErrorKind::Git
        | RepositoryErrorKind::InjectedFailure => ResultCode::InternalError,
        // Shared keys: no ticket command returns these. The task that
        // binds the key commands gives them their codes.
        RepositoryErrorKind::InvalidSharedKeyMetadata
        | RepositoryErrorKind::InvalidSharedKeySourcePath
        | RepositoryErrorKind::SharedKeyRegistryUnavailable
        | RepositoryErrorKind::SharedKeyMaterialPending => ResultCode::InternalError,
        // Remotes: likewise, for the task that binds the remote commands.
        RepositoryErrorKind::InvalidPublicationRemote
        | RepositoryErrorKind::UnavailablePublicationRemote
        | RepositoryErrorKind::SelectedRemoteRemoval
        | RepositoryErrorKind::RemoteNameConflict => ResultCode::InternalError,
        // Enablement: likewise, for the task that binds `repo create` and
        // `repo enable`.
        RepositoryErrorKind::RegistryRefreshPending => ResultCode::InternalError,
    }
}

/// What a save returned without an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveReport {
    /// No identity to commit as: nothing was written.
    IdentityRequired,
    Saved {
        /// The commit the domain names. It is this request's only if the
        /// evidence check says so: a save that changed nothing can name
        /// the head it found.
        claimed: Option<git2::Oid>,
        /// Whether the index hand-off completed.
        discovered: bool,
    },
}

impl SaveReport {
    pub(crate) fn of(outcome: SaveOutcome) -> Self {
        match outcome {
            SaveOutcome::IdentityRequired { context: _ } => Self::IdentityRequired,
            SaveOutcome::Saved {
                context: _,
                checkpoint,
            } => Self::of_checkpoint(checkpoint, true),
            SaveOutcome::IndexPending {
                context: _,
                checkpoint,
            } => Self::of_checkpoint(checkpoint, false),
        }
    }

    pub(crate) fn of_checkpoint(checkpoint: LocalCheckpoint, discovered: bool) -> Self {
        let claimed = match checkpoint {
            LocalCheckpoint::Checkpointed { commit_oid }
            | LocalCheckpoint::RefreshPending { commit_oid } => Some(commit_oid),
            LocalCheckpoint::NoChange => None,
        };
        Self::Saved {
            claimed,
            discovered,
        }
    }

    /// The commit the domain names, for the evidence check to confirm.
    pub(crate) fn claimed(self) -> Option<git2::Oid> {
        match self {
            Self::IdentityRequired => None,
            Self::Saved { claimed, .. } => claimed,
        }
    }

    /// The code and effects of the save, given the commit the evidence
    /// check found to be this request's.
    pub(crate) fn result(self, commit: Option<git2::Oid>) -> (ResultCode, Effects) {
        let discovered = match self {
            Self::IdentityRequired => {
                return (ResultCode::IdentityRequired, Effects::not_requested());
            }
            Self::Saved { discovered, .. } => discovered,
        };
        let effects = Effects {
            write: match commit {
                Some(_) => WriteEffect::Written,
                None => WriteEffect::Unchanged,
            },
            checkpoint: match commit {
                Some(_) => CheckpointEffect::Committed,
                None => CheckpointEffect::Unchanged,
            },
            discovery: match (discovered, commit) {
                (false, _) => DiscoveryEffect::Pending,
                (true, Some(_)) => DiscoveryEffect::Current,
                (true, None) => DiscoveryEffect::NotRequested,
            },
            commit_oid: commit.map(|commit| commit.to_string()),
            ..Effects::not_requested()
        };
        let code = match (discovered, commit) {
            (false, _) => ResultCode::DiscoveryPending,
            (true, Some(_)) => ResultCode::Ok,
            (true, None) => ResultCode::AlreadyApplied,
        };
        (code, effects)
    }
}

/// The effects of a save that stopped with an error while its operation
/// was in flight, from what the repository shows: `commit` is the commit
/// in range that left the file as the request intended, and `written` says
/// the file in the worktree is what the request intended and is not what
/// it was before the request.
pub(crate) fn stopped_save_effects(commit: Option<git2::Oid>, written: bool) -> Effects {
    match commit {
        Some(commit) => Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Committed,
            discovery: DiscoveryEffect::Pending,
            commit_oid: Some(commit.to_string()),
            ..Effects::not_requested()
        },
        None if written => Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Pending,
            discovery: DiscoveryEffect::Pending,
            ..Effects::not_requested()
        },
        None => Effects::not_requested(),
    }
}

#[cfg(test)]
#[path = "outcome_tests.rs"]
mod tests;
