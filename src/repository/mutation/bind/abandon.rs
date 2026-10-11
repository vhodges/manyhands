//! `operation abandon`: a pending local operation is closed, so that it
//! no longer refuses every other operation on its repository. It needs
//! confirmation.

use std::path::{Path, PathBuf};

use crate::results::{ResultCode, Scope};

use super::{
    super::{
        super::{
            AbandonOperationOutcome, AbandonOperationRequest, AbandonableOperation,
            OperationFamily, OperationId, OperationRemains, PendingStanding, RepositoryService,
            read::local_action, recovery::JournalRow,
        },
        command::Command,
        dto::{
            MutationDataDto, OperationAbandonPreviewDto, OperationMutationDto, PreviewDetailsDto,
            PreviewEffect,
        },
        evidence::Position,
        identity::{IntentDigestBuilder, ScopeKey},
        observe::{Observation, ObservationDigest},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused,
};

/// The input of `operation abandon`: the root of the repository, and the
/// local operation of it to close.
#[derive(Clone)]
pub struct OperationAbandonInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
}

/// `operation abandon`: one pending row of the repository's local
/// journal is closed, and the unfinished request that began it released.
/// It needs confirmation. No file under the root and nothing Git holds
/// is touched, so no effect shows it: its data names the operation and
/// what the operation left on disk, and the outcome says whether
/// anything changed.
///
/// **What its preview covers.** The operation's row: that it is pending,
/// and the state and step it stands in. A row that has completed,
/// advanced or gone since is `external_change`. The command reads no
/// identity.
///
/// **Its record holds no operation.** The operation it names is another
/// request's and is the request's target, so the same request ID with
/// another operation is `request_mismatch`. The abandon itself is one
/// transaction under the repository's leases: it happened or it did
/// not. What shows it to be done is the row itself, closed as abandoned.
///
/// **It is not refused by the row it closes.** A pending row refuses
/// every operation that begins a row of its own. An abandon begins none,
/// and closes only the row its target names: that is the whole of the
/// exemption, and every other command stays refused.
pub(crate) struct OperationAbandonBinding {
    root: PathBuf,
    operation_id: OperationId,
    /// How the row stood when this call observed it pending: what the
    /// domain call is told to expect, so that a row that moved between
    /// the observation and the call is left alone.
    seen: Option<PendingStanding>,
}

impl From<OperationAbandonInput> for OperationAbandonBinding {
    fn from(input: OperationAbandonInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let OperationAbandonInput { root, operation_id } = input;
        Self {
            root,
            operation_id,
            seen: None,
        }
    }
}

/// The request's own journal row: it has no operation, so none.
const NO_ROW: JournalRow = JournalRow::Absent;

impl OperationAbandonBinding {
    /// The scope key of the request's root. `None` for a root that
    /// cannot be resolved.
    fn scope_key(&self) -> Option<ScopeKey> {
        match self.place() {
            Place::Repository(root) => ScopeKey::for_repository(root),
            Place::NewRepository(root) => ScopeKey::for_new_repository(root),
            Place::Application => None,
        }
    }

    /// Reads how the operation stands. Nothing is written and no lease
    /// is taken.
    fn observe(
        &self,
        service: &RepositoryService,
    ) -> Result<(ScopeKey, AbandonableOperation), Answer> {
        let scope = self
            .scope_key()
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let found = service
            .abandonable_operation(Path::new(scope.as_str()), self.operation_id)
            .map_err(|error| Answer::of_read(&error))?;
        Ok((scope, found))
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with. It binds how the row
    /// stands, and for a pending row its state and step.
    fn digest(found: &AbandonableOperation) -> ObservationDigest {
        let command = Command::OperationAbandon.name();
        let observation = Observation::of(command);
        let observation = match found {
            AbandonableOperation::NotFound => observation.text("standing", "not_found"),
            AbandonableOperation::Completed => observation.text("standing", "completed"),
            AbandonableOperation::Rejected => observation.text("standing", "rejected"),
            AbandonableOperation::Abandoned => observation.text("standing", "abandoned"),
            AbandonableOperation::Pending(remains) => {
                let PendingStanding { state, step } = &remains.standing;
                let observation = observation.text("standing", "pending").text("state", state);
                match step {
                    Some(step) => observation.text("step", step),
                    None => observation.absent("step"),
                }
            }
        };
        ObservationDigest::of_state(command, observation.finish())
    }

    /// Whether the remote journal of the repository, or the application's
    /// key journal, holds an operation of the ID: one this command does
    /// not abandon.
    fn is_of_another_journal(
        &self,
        service: &RepositoryService,
        scope: &ScopeKey,
    ) -> Result<bool, Answer> {
        for family in [OperationFamily::Remote, OperationFamily::KeyMaterial] {
            let row = service
                .journal_row(family, scope, self.operation_id)
                .map_err(|error| Answer::of_read(&error))?;
            if row != JournalRow::Absent {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn data(&self, remains: OperationRemains) -> MutationDataDto {
        let OperationRemains {
            action,
            standing: PendingStanding { state, step },
            item_id,
            worktree,
            paths,
            paths_complete,
            owner,
        } = remains;
        MutationDataDto::Operation(OperationMutationDto {
            operation_id: self.operation_id.to_string(),
            action: local_action(&action),
            state,
            step,
            item_id,
            worktree,
            paths,
            paths_complete,
            released_request_id: owner,
        })
    }

    /// The one domain call, and what it came to. An abandon is one
    /// transaction: it happened or it did not.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        // The request was accepted, or entered again, on a row this
        // call saw pending. Without that there is nothing the domain
        // could be told to expect, and nothing is closed on that.
        let Some(seen) = self.seen.clone() else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped { journal: None },
            };
        };
        let result = service.abandon_operation(AbandonOperationRequest {
            root: accepted.root(),
            operation_id: self.operation_id,
            expected: Some(seen),
        });
        match result {
            Ok(AbandonOperationOutcome::Abandoned(remains)) => Ran {
                answer: Answer::stopped(ResultCode::Ok).with_data(self.data(remains)),
                standing: Standing::Final,
            },
            // Another request abandoned it meanwhile: the end state
            // holds, and this request changed nothing.
            Ok(AbandonOperationOutcome::AlreadyAbandoned) => Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            },
            // The row is not what was previewed any more: it ended,
            // advanced or went between the observation and the call.
            // Nothing was changed and nothing of this request is in
            // flight.
            Ok(
                AbandonOperationOutcome::NotFound
                | AbandonOperationOutcome::Changed
                | AbandonOperationOutcome::Completed
                | AbandonOperationOutcome::Rejected,
            ) => Ran {
                answer: Answer::stopped(ResultCode::ExternalChange),
                standing: Standing::Stopped {
                    journal: Some(NO_ROW),
                },
            },
            // A call is at work under the row, or the journal could not
            // be read or written. The transaction closed the row or it
            // did not, and the record is kept for the same request to
            // find out: it observes again before it does anything. Only
            // the error's kind is read.
            Err(error) => Ran {
                answer: refused(error),
                standing: Standing::Stopped { journal: None },
            },
        }
    }
}

impl Binding for OperationAbandonBinding {
    fn command(&self) -> Command {
        Command::OperationAbandon
    }

    /// The request has no operation, so no row of its own is ever read.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    /// The repository whose journal holds the operation. A creation that
    /// was killed before it made its directory leaves a row for a root
    /// that is not there: such a root is named as the creation named it.
    fn place(&self) -> Place<'_> {
        if self.root.exists() {
            Place::Repository(&self.root)
        } else {
            Place::NewRepository(&self.root)
        }
    }

    /// The operation to abandon.
    fn target(&self) -> String {
        self.operation_id.to_string()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with: the scope and the target.
            // The request holds nothing else.
            root: _,
            operation_id: _,
            // What a call observed, and no part of the request.
            seen: _,
        } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Observes. No check is made of what the preview covers: the
    /// boundary compares the digest with the confirmation's, and refuses
    /// a request whose preview is stale as `external_change`.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (_, found) = self.observe(call.service)?;
        let observation = Self::digest(&found).to_stored();
        if let AbandonableOperation::Pending(remains) = found {
            self.seen = Some(remains.standing);
        }
        Ok(Prepared {
            // One transaction, with no operation ID and no row.
            family: None,
            // Nothing is committed, so there is no branch to bound.
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
            observation: Some(observation),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// The row the request names is asked first:
    ///
    /// 1. It is closed as abandoned. The request's end state holds, by
    ///    its own work or another's: the row does not say which. The
    ///    request is a no-op and is finished, and is never answered as
    ///    `external_change`.
    /// 2. It is pending as it was previewed: nothing of the request has
    ///    been done, and the domain is called.
    /// 3. Anything else: it ended, advanced or went. What is there is
    ///    not what was shown. Nothing of this request is in flight, so
    ///    its record is deleted and its confirmation released.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        // No row is read for a request with no operation. Nothing is run
        // or settled on one that is.
        if journal != NO_ROW {
            return Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            });
        }
        let found = match self.observe(call.service) {
            Ok((_, found)) => found,
            // Nothing is known of the row: the record is kept.
            Err(answer) => {
                return Err(Ran {
                    answer,
                    standing: Standing::Stopped { journal: None },
                });
            }
        };
        let changed = || Ran {
            answer: Answer::stopped(ResultCode::ExternalChange),
            standing: Standing::NotRun { journal: NO_ROW },
        };
        match &found {
            AbandonableOperation::Abandoned => Err(Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            }),
            AbandonableOperation::Pending(remains) => {
                // A request whose record holds no consent that can be
                // read has none that anything could be compared with:
                // nothing is run.
                if call.previewed() != Some(Self::digest(&found)) {
                    return Err(changed());
                }
                self.seen = Some(remains.standing.clone());
                Ok(Entered::Unfinished)
            }
            AbandonableOperation::NotFound
            | AbandonableOperation::Completed
            | AbandonableOperation::Rejected => Err(changed()),
        }
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

impl Confirmed for OperationAbandonBinding {
    /// A preview is given only of a pending local operation. An ID no
    /// journal of the repository holds is `operation_not_found`; one the
    /// remote journal or the key journal holds is `invalid_input`, since
    /// this command does not abandon those; and a local operation that
    /// has ended, however it ended, is a no-op.
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let (scope, found) = self.observe(call.service)?;
        let observation = Self::digest(&found).to_stored();
        let remains = match found {
            AbandonableOperation::Pending(remains) => remains,
            AbandonableOperation::NotFound => {
                return Err(Answer::stopped(
                    if self.is_of_another_journal(call.service, &scope)? {
                        ResultCode::InvalidInput
                    } else {
                        ResultCode::OperationNotFound
                    },
                ));
            }
            // The command's end state holds: the operation refuses
            // nothing.
            AbandonableOperation::Completed
            | AbandonableOperation::Rejected
            | AbandonableOperation::Abandoned => {
                return Err(Answer::stopped(ResultCode::AlreadyApplied));
            }
        };
        let OperationRemains {
            action,
            standing: PendingStanding { state, step },
            item_id,
            worktree,
            paths,
            paths_complete,
            owner,
        } = remains;
        let mut effects = vec![PreviewEffect::OperationClosed];
        if owner.is_some() {
            effects.push(PreviewEffect::RequestReleased);
        }
        Ok(Previewed {
            observation,
            effects,
            details: PreviewDetailsDto::OperationAbandon(OperationAbandonPreviewDto {
                root: scope.as_str().to_owned(),
                operation_id: self.operation_id.to_string(),
                action: local_action(&action),
                state,
                step,
                item_id,
                worktree,
                paths,
                paths_complete,
                request_id: owner,
                files_changed: false,
                git_state_changed: false,
            }),
        })
    }
}
