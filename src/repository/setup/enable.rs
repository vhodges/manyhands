//! A pull-request `repo enable` (design Part E, "The pull-request path"):
//! the repository is registered with no configuration, its setup is
//! recorded in one registry transaction with the request's local row
//! completed at `setup_recorded`, and the setup is published through the
//! session. A recorded request that is entered again does not record again:
//! it writes what a crash may have left unwritten and publishes.

use rusqlite::TransactionBehavior;

use super::{
    intent::Intended,
    lock::read_record,
    publish::{Published, SetupError},
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical::ItemId,
    repository::{
        CallEffects, CommitIdentity, ConfigurationInspection, EnableRepositoryRequest,
        IdentityConfigProvider, RecoveryRecord, RepositoryError, RepositoryErrorKind,
        RepositoryIdentityConfig, RepositoryOperation, RepositoryService, WorktreeState,
        cache_write_guard, canonical_configuration_for, canonical_repository_root,
        checked_out_branch, ensure_worktree_exclusion,
        mutation::{RequestId, repository_error_code},
        publication_remote_eligible, read_configuration_for,
        recovery::{self, LocalStep},
        register_setup_pending, registry_root_key,
        remote::observation::endpoint_digest_of,
        resolve_identity, validate_identity, worktree_state, write_local_identity,
    },
    results::ResultCode,
};

/// What publishing answers: `None` when the session has no credentials.
pub(crate) type Publishing = Option<Result<Published, SetupError>>;

/// What a pull-request enable asks of the domain, besides the repository's
/// own request.
pub(crate) struct SetupRequest {
    /// The request that records the setup, and continues it.
    pub(crate) request_id: RequestId,
    /// The short code prefix the configuration holds.
    pub(crate) prefix: Option<String>,
    /// The remote the setup is published to.
    pub(crate) remote_name: String,
    /// What a first call records; a recorded request has none to give.
    pub(crate) intended: Option<Intended>,
    /// The request's own local row completed at `setup_recorded`: the setup
    /// is recorded, and nothing is checked or recorded again.
    pub(crate) recorded: bool,
}

/// How far a pull-request enable got, once it passed its checks and made its
/// registry transaction.
#[derive(Debug)]
pub(crate) enum SetupOutcome {
    /// The remote holds the setup's branch.
    Published(Published),
    /// The setup is recorded and publishing stopped, with `stop`: the request
    /// is owed, and goes on from its record.
    /// `authority` is the host a transport stop is about, as a recovery
    /// action names it.
    Recorded {
        stop: ResultCode,
        authority: Option<String>,
    },
    /// The setup ended: the remote refused it, or something removed it, and
    /// the request was finished.
    Ended(SetupError),
    /// Another request's setup is recorded for the repository.
    AnotherSetup,
    /// No identity to commit as: nothing was recorded.
    IdentityRequired,
}

/// The host a transport stop is about, as a recovery action names it:
/// `host:port`, with an address that holds a colon in brackets.
fn host_of(error: &SetupError) -> Option<String> {
    let SetupError::Transport(error) = error else {
        return None;
    };
    let authority = error.authority.as_ref()?;
    Some(if authority.host.contains(':') {
        format!("[{}]:{}", authority.host, authority.port)
    } else {
        format!("{}:{}", authority.host, authority.port)
    })
}

/// Where the checks and the recording left the call.
enum Stage {
    Found(Found),
    /// The call ends, with nothing recorded.
    Stop(Box<SetupOutcome>),
}

/// What the recording transaction found.
enum Found {
    /// This call recorded the setup.
    Recorded,
    /// This request had recorded it.
    Own,
    /// Another request's setup is there: this request's is not.
    Another,
    /// A recorded request finds no setup of its own: it ended, or another's
    /// stands. Publishing answers what became of the request.
    Gone,
}

impl RepositoryService {
    /// Enables the repository for a pull request: the steps of design Part E.
    ///
    /// Under the lifecycle's lease and the request's own local row: the
    /// checks of an enable, then one registry transaction that registers the
    /// repository with no configuration, records the setup, and completes
    /// the row at `setup_recorded`; then the supplied identity and the
    /// worktree exclusion, still under the lease; then the lease is released
    /// and the setup is published through `publish`, the session's, never with a lease held
    /// across a connection.
    ///
    /// An `Err` is a refusal before the recording, settled as an enable's.
    /// From the recording on every outcome is an `Ok`.
    pub(crate) fn enable_with_setup(
        &self,
        request: EnableRepositoryRequest,
        setup: SetupRequest,
        publish: &dyn Fn(&std::path::Path, RequestId) -> Publishing,
    ) -> Result<SetupOutcome, RepositoryError> {
        let operation = RepositoryOperation::Enable;
        self.require_index_available(operation, Some(&request.root))?;
        let (repository, root) = canonical_repository_root(&request.root, operation)?;
        registry_root_key(&root, operation)?;
        let (lease, record) = self.begin_lifecycle(
            &repository,
            &root,
            operation,
            request.operation_id,
            &request.primary_branch,
        )?;
        let effects = CallEffects::default();
        let found = self.settle_rejected_lifecycle(
            &root,
            operation,
            record,
            &effects,
            self.find_or_record(&request, &setup, &repository, &root, record),
        )?;
        let found = match found {
            Stage::Found(found) => found,
            Stage::Stop(outcome) => {
                drop(lease);
                return Ok(*outcome);
            }
        };
        // The setup is recorded: nothing refuses it now. What is left to
        // write is the request's own supplied identity and the exclusion.
        if let Found::Another = found {
            self.complete_lifecycle(&root, operation, record)?;
            drop(lease);
            return Ok(SetupOutcome::AnotherSetup);
        }
        let written = self.write_what_is_left(&request, &repository, &root, &found, &setup);
        drop(lease);
        if let Err(error) = written {
            return Ok(SetupOutcome::Recorded {
                stop: repository_error_code(error.kind),
                authority: None,
            });
        }
        Ok(match publish(&root, setup.request_id) {
            None => SetupOutcome::Recorded {
                stop: ResultCode::InternalError,
                authority: None,
            },
            Some(Ok(published)) => SetupOutcome::Published(published),
            Some(Err(error)) => match error {
                SetupError::RemotePrimaryMissing
                | SetupError::PrimaryHasLocalCommits { .. }
                | SetupError::RemoteAlreadySetUp
                | SetupError::ExternalChange
                | SetupError::Ended(_) => SetupOutcome::Ended(error),
                other => SetupOutcome::Recorded {
                    stop: other.code(),
                    authority: host_of(&other),
                },
            },
        })
    }

    /// The checks, and the recording transaction. An outer `Err` is a
    /// refusal before the recording; the inner one is a call that ends
    /// without recording and is not a refusal.
    fn find_or_record(
        &self,
        request: &EnableRepositoryRequest,
        setup: &SetupRequest,
        repository: &git2::Repository,
        root: &std::path::Path,
        record: RecoveryRecord,
    ) -> Result<Stage, RepositoryError> {
        let operation = RepositoryOperation::Enable;
        if setup.recorded {
            // A record that cannot be read is owed, and is not found absent.
            let own = read_record(self, root)
                .map_err(|error| match error {
                    SetupError::Repository(error) => error,
                    _ => crate::repository::remote::state::recovery_required(),
                })?
                .is_some_and(|(_, record)| record.request_id == setup.request_id);
            return Ok(Stage::Found(if own { Found::Own } else { Found::Gone }));
        }
        self.check_before_recording(request, setup, repository, root)?;
        // An identity that the request does not supply must be there.
        if request.identity.is_none() && !self.has_identity(repository, root)? {
            self.complete_lifecycle(root, operation, record)?;
            return Ok(Stage::Stop(Box::new(SetupOutcome::IdentityRequired)));
        }
        self.record_setup(request, setup, repository, root, record)
            .map(Stage::Found)
    }

    /// Today's checks of an enable, for a repository with no configuration.
    fn check_before_recording(
        &self,
        request: &EnableRepositoryRequest,
        setup: &SetupRequest,
        repository: &git2::Repository,
        root: &std::path::Path,
    ) -> Result<(), RepositoryError> {
        let operation = RepositoryOperation::Enable;
        let refuse = |kind, message: &str| {
            Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                kind,
                message,
            ))
        };
        // An unborn HEAD is refused: activation fast-forwards an existing
        // branch (R-11).
        match repository.head() {
            Ok(_) => {
                let branch = checked_out_branch(repository, root, operation)?;
                if branch != request.primary_branch {
                    return refuse(
                        RepositoryErrorKind::WrongCheckedOutBranch,
                        "the requested primary branch is not checked out at the repository root",
                    );
                }
            }
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {
                return refuse(
                    RepositoryErrorKind::WrongCheckedOutBranch,
                    "a pull request needs a primary branch that has a commit",
                );
            }
            Err(error) => {
                return Err(RepositoryError::git(
                    operation,
                    Some(root.to_owned()),
                    error,
                ));
            }
        }
        match worktree_state(repository, root)? {
            WorktreeState::Clean => {}
            WorktreeState::Dirty => {
                return refuse(
                    RepositoryErrorKind::DirtyWorktree,
                    "initialization requires a clean worktree",
                );
            }
            WorktreeState::Conflicted => {
                return refuse(
                    RepositoryErrorKind::ConflictedWorktree,
                    "initialization requires a non-conflicted worktree",
                );
            }
        }
        match read_configuration_for(root, operation)? {
            ConfigurationInspection::Missing => {}
            ConfigurationInspection::Invalid(problem) => {
                return refuse(RepositoryErrorKind::InvalidConfiguration, &problem.message);
            }
            // The binding chose this path for a repository with none.
            ConfigurationInspection::Valid(_) => {
                return refuse(
                    RepositoryErrorKind::ExternalChange,
                    "the repository holds a configuration already",
                );
            }
        }
        if !publication_remote_eligible(repository, root, &setup.remote_name)? {
            return refuse(
                RepositoryErrorKind::UnavailablePublicationRemote,
                "the publication remote is unavailable or ineligible",
            );
        }
        if let Some(identity) = &request.identity {
            validate_identity(identity, operation)?;
        }
        Ok(())
    }

    /// Whether Git has an identity to commit as.
    fn has_identity(
        &self,
        repository: &git2::Repository,
        root: &std::path::Path,
    ) -> Result<bool, RepositoryError> {
        let operation = RepositoryOperation::Enable;
        let local = repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
        let effective = RepositoryIdentityConfig.effective_config(repository, root, operation)?;
        Ok(resolve_identity(&local, &effective, root, operation)?.is_some())
    }

    /// The recording transaction (design K1): under the registry's write
    /// guard, finds the repository's record and either makes this request's
    /// or leaves another's.
    fn record_setup(
        &self,
        request: &EnableRepositoryRequest,
        setup: &SetupRequest,
        repository: &git2::Repository,
        root: &std::path::Path,
        record: RecoveryRecord,
    ) -> Result<Found, RepositoryError> {
        let operation = RepositoryOperation::Enable;
        self.check_failure(
            crate::repository::FailurePoint::BeforeRegistryWrite,
            operation,
            root,
        )?;
        // The hook is taken out of its mutex before it runs, so that one
        // that panics, as a test's crash does, poisons nothing.
        let hook = self
            .registration_git_hook
            .lock()
            .map_err(|_| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InjectedFailure,
                    "the test registration Git hook is unavailable",
                )
            })?
            .take();
        if let Some(hook) = hook {
            hook();
        }
        let Some(intended) = &setup.intended else {
            return Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::ExternalChange,
                "a setup to record names no intent",
            ));
        };
        let configuration = canonical_configuration_for(
            intended.configuration.clone(),
            setup.prefix.as_deref(),
            root,
            operation,
        )?
        .into_bytes();
        let endpoint_digest = endpoint_digest_of(repository, &setup.remote_name);
        let now = self.clock.now().unix_timestamp();
        let _guard = cache_write_guard(&self.registry_path, root, operation)?;
        let mut connection = self
            .open_migrated_registry(operation)
            .map_err(|error| error.for_operation(operation, root))?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(RepositoryError::sqlite)?;
        // The repository's record first: nothing is written for another's.
        let key = registry_root_key(root, operation)?;
        let existing = tx
            .query_row(
                "SELECT id FROM repositories WHERE root_path = ?1",
                [key],
                |row| row.get::<_, i64>(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                error => Err(error),
            })
            .map_err(RepositoryError::sqlite)?;
        if let Some(id) = existing
            && let Some(found) = record::read(&tx, id)?
        {
            return Ok(if found.request_id == setup.request_id {
                Found::Own
            } else {
                Found::Another
            });
        }
        let repository_id = register_setup_pending(&tx, root)?;
        record::insert(
            &tx,
            repository_id,
            &SetupRecord {
                setup_id: ItemId::generate(),
                kind: SetupKind::Enable,
                remote_name: setup.remote_name.clone(),
                primary_branch: request.primary_branch.clone(),
                endpoint_digest,
                configuration,
                base_configuration_oid: None,
                base_oid: None,
                commit_oid: None,
                published_oid: None,
                request_id: setup.request_id,
                created_at: now,
            },
        )?;
        recovery::complete_with_step(&tx, record.id, LocalStep::SetupRecorded)?;
        tx.commit().map_err(RepositoryError::sqlite)?;
        Ok(Found::Recorded)
    }

    /// The supplied identity and the worktree exclusion, written as an
    /// enable writes them. Written again on a re-entry, from the request's
    /// own input, for a crash may have come between the recording and them.
    fn write_what_is_left(
        &self,
        request: &EnableRepositoryRequest,
        repository: &git2::Repository,
        root: &std::path::Path,
        found: &Found,
        _setup: &SetupRequest,
    ) -> Result<(), RepositoryError> {
        if !matches!(found, Found::Recorded | Found::Own) {
            return Ok(());
        }
        let identity: Option<&CommitIdentity> = request.identity.as_ref();
        if let Some(identity) = identity {
            write_local_identity(repository, root, identity)?;
        }
        ensure_worktree_exclusion(repository, root)
    }
}
