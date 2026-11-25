//! A pull-request `remote select` (design Part F): which way a change of the
//! publication remote, the mode or the forge goes, and its recording.
//!
//! `path_of` is the table of Part F over the primary's configuration alone.
//! `change_setup` is the pull-request path (`enable_with_setup`'s twin): the
//! checks of a selection, then one registry transaction that finds no other
//! setup and nothing in the change's way (Part B's race rule), records the
//! change and completes the request's local row at `setup_recorded`, then the
//! lease is released and the change is published through the session.

use std::path::PathBuf;

use rusqlite::{OptionalExtension, TransactionBehavior, params};

use super::{
    enable::{Publishing, SetupOutcome, outcome_of},
    intent::{Intended, apply_forge},
    lock::read_record,
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical::{self, ForgeSetting, Integration, ItemId, RepositoryConfig},
    repository::{
        Blocker, CallEffects, ConfigurationInspection, OperationFamily, OperationId,
        RecoveryRecord, RepositoryError, RepositoryErrorKind, RepositoryOperation,
        RepositoryService, cache_write_guard, canonical_repository_root, checked_out_branch,
        configuration_entry_from_tree, ensure_configuration_path_clean,
        mutation::{RequestId, command::Command},
        publication_remote_eligible_for, read_configuration_for,
        recovery::{self, LocalStep},
        registry_root_key,
        remote::observation::endpoint_digest_of,
        resolve_identity,
    },
    results::ResultCode,
};

/// What a `remote select` asks of the configuration.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Change<'a> {
    /// The remote to select; `None` clears the selection.
    pub(crate) name: Option<&'a str>,
    pub(crate) integration: Option<Integration>,
    /// The outer `None` leaves `[forge]` as it is, `Some(None)` removes it.
    pub(crate) forge: Option<Option<&'a ForgeSetting>>,
}

/// The file a change puts on a setup branch, and the configuration it was
/// derived from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Planned {
    pub(crate) intended: Intended,
    pub(crate) from: RepositoryConfig,
}

/// Which way a change goes (design Part F's table).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ChangePath {
    AlreadyApplied,
    Direct,
    Setup(Box<Planned>),
}

/// Why a change is refused before any other check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    InvalidInput,
    InvalidConfiguration,
}

impl Refused {
    pub(crate) const fn code(self) -> ResultCode {
        match self {
            Self::InvalidInput => ResultCode::InvalidInput,
            Self::InvalidConfiguration => ResultCode::InvalidConfiguration,
        }
    }
}

/// The way `change` goes for the repository whose primary holds `current`
/// (design Part F's table).
pub(crate) fn path_of(
    current: &RepositoryConfig,
    change: &Change<'_>,
) -> Result<ChangePath, Refused> {
    let current_mode = current.integration();
    // The intended configuration: the primary's, with the remote, and the
    // mode and the forge where given. The mode changes only when it is a
    // change, so a file with no `integration` key stays with none.
    let mut intended = current.clone();
    intended.publication_remote = change.name.map(str::to_owned);
    match change.integration {
        Some(Integration::Direct) if current_mode != Integration::Direct => {
            intended.unknown.remove("integration");
        }
        Some(Integration::PullRequest) if current_mode != Integration::PullRequest => {
            intended.unknown.insert(
                "integration".to_owned(),
                toml::Value::String(Integration::PullRequest.as_str().to_owned()),
            );
        }
        Some(_) | None => {}
    }
    apply_forge(&mut intended, change.forge);
    // Validation comes first, as today: a setup cannot be published without
    // a remote, so clearing the remote of a pull-request configuration is
    // refused whatever the mode asked for.
    if (current_mode == Integration::PullRequest && change.name.is_none())
        || canonical::serialize_repository_config(&intended).is_err()
    {
        return Err(Refused::InvalidConfiguration);
    }
    // Decision 8: a pull-request configuration keeps its remote.
    if current_mode == Integration::PullRequest
        && current.publication_remote.as_deref() != change.name
    {
        return Err(Refused::InvalidInput);
    }
    if intended == *current {
        return Ok(ChangePath::AlreadyApplied);
    }
    if current_mode == Integration::Direct && intended.integration() == Integration::Direct {
        return Ok(ChangePath::Direct);
    }
    Ok(ChangePath::Setup(Box::new(Planned {
        intended: Intended {
            integration: intended.integration(),
            configuration: intended,
        },
        from: current.clone(),
    })))
}

/// What a pull-request `remote select` asks of the domain.
pub(crate) struct ChangeSetupRequest {
    pub(crate) root: PathBuf,
    pub(crate) operation_id: OperationId,
    pub(crate) request_id: RequestId,
    /// What a first call records; a recorded request has none to give.
    pub(crate) planned: Option<Planned>,
    /// The request's own local row completed at `setup_recorded`: the setup
    /// is recorded, and nothing is checked or recorded again.
    pub(crate) recorded: bool,
}

/// What the recording transaction found.
enum Found {
    /// This call recorded the change.
    Recorded,
    /// This request had recorded it.
    Own,
    /// A recorded request finds no setup of its own: it ended, or another's
    /// stands. Publishing answers what became of the request.
    Gone,
    /// Another request's setup is recorded.
    Another,
    /// Something is in the change's way.
    InTheWay(Option<Blocker>),
}

impl RepositoryService {
    /// Puts a change of the publication remote, the mode or the forge on a
    /// setup branch: the steps of design Part F. An `Err` is a refusal before
    /// the recording, settled as a selection's; from the recording on every
    /// outcome is an `Ok`.
    pub(crate) fn change_setup(
        &self,
        request: ChangeSetupRequest,
        publish: &dyn Fn(&std::path::Path, RequestId) -> Publishing,
    ) -> Result<SetupOutcome, RepositoryError> {
        let operation = RepositoryOperation::SetPublicationRemote;
        self.require_index_available(operation, Some(&request.root))?;
        let (repository, root) = canonical_repository_root(&request.root, operation)?;
        registry_root_key(&root, operation)?;
        // The row's target is the remote: a recorded request names it by its
        // record.
        let target = match &request.planned {
            Some(planned) => planned
                .intended
                .configuration
                .publication_remote
                .clone()
                .unwrap_or_default(),
            None => read_record(self, &root)
                .ok()
                .flatten()
                .map(|(_, record)| record.remote_name)
                .unwrap_or_default(),
        };
        let (lease, record) =
            self.begin_lifecycle(&repository, &root, operation, request.operation_id, &target)?;
        let effects = CallEffects::default();
        let stage = self.settle_rejected_lifecycle(
            &root,
            operation,
            record,
            &effects,
            self.find_or_record_change(&request, &repository, &root, record),
        )?;
        match stage {
            Stage::Stop(outcome) => {
                drop(lease);
                Ok(*outcome)
            }
            Stage::Found(Found::Another) => {
                self.complete_lifecycle(&root, operation, record)?;
                drop(lease);
                Ok(SetupOutcome::AnotherSetup)
            }
            Stage::Found(Found::InTheWay(blocker)) => {
                self.complete_lifecycle(&root, operation, record)?;
                drop(lease);
                Ok(SetupOutcome::InTheWay { blocker })
            }
            Stage::Found(Found::Recorded | Found::Own | Found::Gone) => {
                // The lease is not held across a connection.
                drop(lease);
                Ok(outcome_of(publish(&root, request.request_id)))
            }
        }
    }

    /// The checks, and the recording transaction.
    fn find_or_record_change(
        &self,
        request: &ChangeSetupRequest,
        repository: &git2::Repository,
        root: &std::path::Path,
        record: RecoveryRecord,
    ) -> Result<Stage, RepositoryError> {
        let operation = RepositoryOperation::SetPublicationRemote;
        if request.recorded {
            // A record that cannot be read is owed, and is not found absent.
            let own = read_record(self, root)
                .map_err(|error| match error {
                    super::publish::SetupError::Repository(error) => error,
                    _ => crate::repository::remote::state::recovery_required(),
                })?
                .is_some_and(|(_, record)| record.request_id == request.request_id);
            return Ok(Stage::Found(if own { Found::Own } else { Found::Gone }));
        }
        let Some(planned) = &request.planned else {
            return Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::ExternalChange,
                "a change to record names no intent",
            ));
        };
        let refuse = |kind, message: &str| {
            Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                kind,
                message,
            ))
        };
        // The checks of a selection, in its order.
        let ConfigurationInspection::Valid(current) = read_configuration_for(root, operation)?
        else {
            return refuse(
                RepositoryErrorKind::InvalidConfiguration,
                "a valid Manyhands configuration is required",
            );
        };
        if checked_out_branch(repository, root, operation)? != current.primary_branch {
            return refuse(
                RepositoryErrorKind::WrongCheckedOutBranch,
                "the configured primary branch must be checked out",
            );
        }
        ensure_configuration_path_clean(repository, root, operation)?;
        let Some(remote) = planned.intended.configuration.publication_remote.as_deref() else {
            return refuse(
                RepositoryErrorKind::InvalidConfiguration,
                "a setup needs a publication remote",
            );
        };
        match repository.find_remote(remote) {
            Ok(_) => {}
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                return refuse(
                    RepositoryErrorKind::UnavailablePublicationRemote,
                    "the selected publication remote does not exist",
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
        if !publication_remote_eligible_for(repository, root, remote, operation)? {
            return refuse(
                RepositoryErrorKind::InvalidPublicationRemote,
                "the selected remote must have SSH-compatible fetch and push URLs",
            );
        }
        // The file was derived from a primary that has moved since.
        if current != planned.from {
            return refuse(
                RepositoryErrorKind::ExternalChange,
                "the configuration changed since the change was planned",
            );
        }
        let local = repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
        if resolve_identity(&local, &local, root, operation)?.is_none() {
            self.complete_lifecycle(root, operation, record)?;
            return Ok(Stage::Stop(Box::new(SetupOutcome::IdentityRequired)));
        }
        let blob = repository
            .head()
            .and_then(|head| head.peel_to_tree())
            .ok()
            .and_then(|tree| configuration_entry_from_tree(&tree))
            .map(|(oid, _)| oid)
            .ok_or_else(|| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InvalidConfiguration,
                    "the primary holds no configuration",
                )
            })?;
        self.record_change(request, planned, repository, root, record, remote, blob)
            .map(Stage::Found)
    }

    /// The recording transaction (design K1 and Part B): under the registry's
    /// write guard, finds the repository's record and what is in the change's
    /// way, and either makes this request's record or leaves what it found.
    #[allow(clippy::too_many_arguments)]
    fn record_change(
        &self,
        request: &ChangeSetupRequest,
        planned: &Planned,
        repository: &git2::Repository,
        root: &std::path::Path,
        record: RecoveryRecord,
        remote: &str,
        blob: git2::Oid,
    ) -> Result<Found, RepositoryError> {
        let operation = RepositoryOperation::SetPublicationRemote;
        self.check_failure(
            crate::repository::FailurePoint::BeforeRegistryWrite,
            operation,
            root,
        )?;
        // The hook is taken out of its mutex before it runs, so that one that
        // panics, as a test's crash does, poisons nothing.
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
        let configuration = canonical::serialize_repository_config(&planned.intended.configuration)
            .map_err(|problem| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InvalidConfiguration,
                    problem.message,
                )
            })?
            .into_bytes();
        let endpoint_digest = endpoint_digest_of(repository, remote);
        let now = self.clock.now().unix_timestamp();
        let _guard = cache_write_guard(&self.registry_path, root, operation)?;
        let mut connection = self
            .open_migrated_registry(operation)
            .map_err(|error| error.for_operation(operation, root))?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(RepositoryError::sqlite)?;
        let key = registry_root_key(root, operation)?;
        let repository_id: i64 = tx
            .query_row(
                "SELECT id FROM repositories WHERE root_path = ?1",
                [&key],
                |row| row.get(0),
            )
            .optional()
            .map_err(RepositoryError::sqlite)?
            .ok_or_else(crate::repository::remote::state::recovery_required)?;
        // The repository's record first: nothing is written for another's.
        if let Some(found) = record::read(&tx, repository_id)? {
            return Ok(if found.request_id == request.request_id {
                Found::Own
            } else {
                Found::Another
            });
        }
        // Part B, "Closing the races": no unfinished close or promotion, and
        // no accepted close, promotion or selection but this request's own.
        if let Some(blocker) = in_the_way(&tx, repository_id, key, request.request_id)? {
            return Ok(Found::InTheWay(blocker));
        }
        record::insert(
            &tx,
            repository_id,
            &SetupRecord {
                setup_id: ItemId::generate(),
                kind: SetupKind::Change,
                remote_name: remote.to_owned(),
                primary_branch: planned.from.primary_branch.clone(),
                endpoint_digest,
                configuration,
                base_configuration_oid: Some(blob),
                base_oid: None,
                commit_oid: None,
                published_oid: None,
                request_id: request.request_id,
                created_at: now,
            },
        )?;
        recovery::complete_with_step(&tx, record.id, LocalStep::SetupRecorded)?;
        tx.commit().map_err(RepositoryError::sqlite)?;
        Ok(Found::Recorded)
    }
}

/// Where the checks and the recording left the call.
enum Stage {
    Found(Found),
    /// The call ends, with nothing recorded.
    Stop(Box<SetupOutcome>),
}

/// What is in the way of a change at `root`: an unfinished close or
/// promotion parent, or an accepted `ticket close`, `document promote` or
/// `remote select` request other than `own`. `Some(blocker)` says something
/// is, and names its operation where either journal holds its row.
fn in_the_way(
    tx: &rusqlite::Transaction<'_>,
    repository_id: i64,
    root: &str,
    own: RequestId,
) -> Result<Option<Option<Blocker>>, RepositoryError> {
    let sqlite = RepositoryError::sqlite;
    let named = |operation: Option<String>| -> Result<Option<Blocker>, RepositoryError> {
        let Some(operation) = operation else {
            return Ok(None);
        };
        let local: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid = ?1)",
                [&operation],
                |row| row.get(0),
            )
            .map_err(RepositoryError::sqlite)?;
        Ok(if local {
            Blocker::local(Some(&operation))
        } else {
            Blocker::remote(&operation)
        })
    };
    for table in ["ticket_closures", "document_promotions"] {
        let parent: Option<String> = tx
            .query_row(
                &format!(
                    "SELECT operation_id FROM {table}
                      WHERE repository_id = ?1
                        AND step NOT IN ('complete','abandoned','superseded')
                      ORDER BY rowid LIMIT 1"
                ),
                [repository_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite)?;
        if parent.is_some() {
            return Ok(Some(named(parent)?));
        }
    }
    let accepted: Option<(String, Option<String>)> = tx
        .query_row(
            "SELECT r.request_ulid,
                    (SELECT o.operation_ulid FROM request_operations o
                      WHERE o.request_ulid = r.request_ulid AND o.family IN (?5, ?6)
                      ORDER BY o.rowid LIMIT 1)
               FROM request_records r
              WHERE r.state = 'accepted' AND r.scope_key = ?1
                AND r.command IN (?2, ?3, ?4) AND r.request_ulid != ?7
              ORDER BY r.rowid LIMIT 1",
            params![
                root,
                Command::TicketClose.name(),
                Command::DocumentPromote.name(),
                Command::RemoteSelect.name(),
                OperationFamily::Local.as_str(),
                OperationFamily::Remote.as_str(),
                own.to_string(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sqlite)?;
    match accepted {
        Some((_, operation)) => Ok(Some(named(operation)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
#[path = "change_tests.rs"]
pub(crate) mod tests;
