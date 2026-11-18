//! `operation abandon`: a pending local operation is closed, so that it
//! no longer refuses every other operation on its repository. It needs
//! confirmation. Retirement is a guarded preserving exception: its controlling
//! request is retained and its discovery debt settles before the blocker closes.
//! A synchronization that no process works under is closed the same way, its
//! push recorded refused, once the person has seen what its row last saw of
//! the remote: a hand-over, never an agent's own step.

use crate::repository::mutation::confirm::{self, Row, Seen};
use std::path::{Path, PathBuf};

use crate::results::{ResultCode, Scope};

use super::{
    super::{
        super::{
            AbandonOperationOutcome, AbandonOperationRequest, AbandonableOperation, Blocker,
            OperationFamily, OperationId, OperationRemains, PendingStanding, RepositoryErrorKind,
            RepositoryService,
            bridges::abandon::{
                AbandonableRemote, RemoteAbandonOutcome, RemoteRemains, RemoteStanding,
            },
            read::{local_action, remote_action, remote_phase, remote_step},
            recovery::JournalRow,
        },
        command::Command,
        dto::{
            MutationDataDto, OperationAbandonEvidenceDto, OperationAbandonPreviewDto,
            OperationMutationDto, PreviewDetailsDto, PreviewEffect,
        },
        evidence::Position,
        identity::{IntentDigestBuilder, ScopeKey},
        observe::{Observation, ObservationDigest},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused, resume,
};

/// The input of `operation abandon`: the root of the repository, and the
/// local operation of it to close.
#[derive(Clone)]
pub struct OperationAbandonInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
}

/// `operation abandon`: one pending row of the repository's local
/// journal is closed, and the unfinished request that began it released (except
/// retirement, whose identity stays bound and preserving index debt is finalized).
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
///
/// **A synchronization.** An ID of the remote journal that
/// `abandon_remote_operation` closes is previewed with what its row last
/// saw of the remote (`evidence`), and with `push_marked_rejected` for a
/// push nothing answered. Its digest binds its phase, step, checkpoint and
/// owner epoch and that evidence. While another process of this host is at
/// work under it, holding the reservation, no preview is given and a
/// confirmed request is `busy`, naming it, with nothing recorded: an
/// operation a live process owns is never taken from it.
pub(crate) struct OperationAbandonBinding {
    root: PathBuf,
    operation_id: OperationId,
    /// How the row stood when this call observed it pending: what the
    /// domain call is told to expect, so that a row that moved between
    /// the observation and the call is left alone.
    seen: Option<PendingStanding>,
    /// The same, for a synchronization's row of the remote journal.
    remote: Option<RemoteStanding>,
    retirement: Option<crate::repository::RetirementAbandonMetadata>,
}

/// What an abandon found under its operation ID.
enum Found {
    /// The local journal's answer: also for an ID another journal holds
    /// that this command does not close, which reads `AnotherJournal`.
    Local(AbandonableOperation),
    /// A synchronization of the remote journal that it closes, or one that
    /// has ended.
    Remote(AbandonableRemote),
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
            remote: None,
            retirement: None,
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
    /// is taken. An ID another journal holds is asked of the remote
    /// journal, which says whether it is a synchronization this command
    /// closes.
    fn observe(&self, service: &RepositoryService) -> Result<(ScopeKey, Found), Answer> {
        let scope = self
            .scope_key()
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let root = Path::new(scope.as_str());
        let found = service
            .abandonable_operation(root, self.operation_id)
            .map_err(|error| Answer::of_read(&error))?;
        if found != AbandonableOperation::AnotherJournal {
            return Ok((scope, Found::Local(found)));
        }
        let remote = service
            .abandonable_remote_operation(root, self.operation_id)
            .map_err(|error| Answer::of_read(&error))?;
        let found = match remote {
            AbandonableRemote::NotFound | AbandonableRemote::Refused => Found::Local(found),
            remote => Found::Remote(remote),
        };
        Ok((scope, found))
    }

    /// The refusal of an abandon of the synchronization this binding names
    /// while another process is still at work under it: `busy`, naming it,
    /// as a resume of it is refused. `None` for one no one works under.
    fn held(&self, service: &RepositoryService, scope: &ScopeKey) -> Option<Answer> {
        resume::held_by_another_process(
            service,
            Path::new(scope.as_str()),
            Blocker {
                family: OperationFamily::Remote,
                operation_id: self.operation_id,
                live: None,
                seen: None,
            },
        )
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with. It binds how the row
    /// stands, and for a pending row its state and step; for a
    /// synchronization, its phase, step, checkpoint, owner epoch and the
    /// evidence its preview shows.
    fn digest(
        found: &Found,
        retirement: Option<&crate::repository::RetirementAbandonMetadata>,
    ) -> ObservationDigest {
        let command = Command::OperationAbandon.name();
        let observation = Observation::of(command);
        let optional = |observation: Observation, name: &str, value: Option<String>| match value {
            Some(value) => observation.text(name, &value),
            None => observation.absent(name),
        };
        let observation = match found {
            Found::Local(AbandonableOperation::NotFound) => {
                observation.text("standing", "not_found")
            }
            Found::Local(AbandonableOperation::AnotherJournal) => {
                observation.text("standing", "another_journal")
            }
            Found::Local(AbandonableOperation::Completed) => {
                observation.text("standing", "completed")
            }
            Found::Local(AbandonableOperation::Rejected) => {
                observation.text("standing", "rejected")
            }
            Found::Local(AbandonableOperation::Abandoned) => {
                observation.text("standing", "abandoned")
            }
            Found::Local(AbandonableOperation::Pending(remains)) => {
                let PendingStanding { state, step } = &remains.standing;
                let observation = observation.text("standing", "pending").text("state", state);
                match step {
                    Some(step) => observation.text("step", step),
                    None => observation.absent("step"),
                }
            }
            Found::Remote(AbandonableRemote::Pending(remains)) => {
                let RemoteStanding {
                    phase,
                    step,
                    checkpoint,
                    owner_epoch,
                } = &remains.standing;
                let evidence = &remains.evidence;
                let observation = observation
                    .text("standing", "remote_pending")
                    .text("phase", remote_phase(*phase).0)
                    .text("owner_epoch", &owner_epoch.to_string());
                let observation = optional(
                    observation,
                    "step",
                    step.map(|step| remote_step(step).to_owned()),
                );
                let observation =
                    optional(observation, "checkpoint", checkpoint.map(str::to_owned));
                let observation = optional(
                    observation,
                    "remote_ref_present",
                    evidence
                        .remote_ref_present
                        .map(|present| present.to_string()),
                );
                let observation = optional(
                    observation,
                    "remote_oid",
                    evidence.remote_oid.map(|oid| oid.to_string()),
                );
                let observation = optional(
                    observation,
                    "local_oid",
                    evidence.local_oid.map(|oid| oid.to_string()),
                );
                optional(
                    observation,
                    "push_oid",
                    evidence.push_oid.map(|oid| oid.to_string()),
                )
            }
            Found::Remote(AbandonableRemote::Ended { cancelled }) => observation.text(
                "standing",
                if *cancelled {
                    "remote_cancelled"
                } else {
                    "remote_completed"
                },
            ),
            // Never found: such an ID reads `AnotherJournal`.
            Found::Remote(AbandonableRemote::NotFound | AbandonableRemote::Refused) => {
                observation.text("standing", "another_journal")
            }
        };
        let observation = if let Some(metadata) = retirement {
            observation.text(
                "retirement",
                &serde_json::to_string(metadata).expect("closed metadata"),
            )
        } else {
            observation
        };
        ObservationDigest::of_state(command, observation.finish())
    }

    fn data(&self, remains: OperationRemains) -> MutationDataDto {
        let OperationRemains {
            action,
            standing: PendingStanding { state, step },
            item_id,
            worktree,
            paths,
            paths_complete,
            committed,
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
            committed,
            released_request_id: owner,
        })
    }

    /// What the abandon of a synchronization names of the operation it
    /// closed: as of a local one, with no worktree or path, since it
    /// writes none, and nothing committed of its own.
    fn remote_data(&self, remains: &RemoteRemains) -> MutationDataDto {
        MutationDataDto::Operation(OperationMutationDto {
            operation_id: self.operation_id.to_string(),
            action: Some(remote_action(remains.action)),
            state: remote_phase(remains.standing.phase).0.to_owned(),
            step: remains
                .standing
                .step
                .map(|step| remote_step(step).to_owned()),
            item_id: remains.item_id.clone(),
            worktree: None,
            paths: Vec::new(),
            paths_complete: true,
            committed: false,
            released_request_id: remains.owner.clone(),
        })
    }

    /// The domain call of the abandon of a synchronization, and what it
    /// came to. It is one transaction: it happened or it did not.
    fn call_remote(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        expected: &RemoteStanding,
    ) -> Ran {
        let root = accepted.root();
        match service.abandon_remote_operation(&root, self.operation_id, Some(expected)) {
            Ok(RemoteAbandonOutcome::Abandoned(remains)) => Ran {
                answer: Answer::stopped(ResultCode::Ok).with_data(self.remote_data(&remains)),
                standing: Standing::Final,
            },
            // Another request closed it meanwhile, or it ended by itself:
            // the end state holds, and this request changed nothing.
            Ok(RemoteAbandonOutcome::AlreadyEnded) => Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            },
            // The row is not what was previewed any more. Nothing was
            // changed and nothing of this request is in flight.
            Ok(
                RemoteAbandonOutcome::NotFound
                | RemoteAbandonOutcome::Refused
                | RemoteAbandonOutcome::Changed,
            ) => Ran {
                answer: Answer::stopped(ResultCode::ExternalChange),
                standing: Standing::Stopped {
                    journal: Some(NO_ROW),
                },
            },
            // A process came back to work under it: nothing was changed,
            // and the request is answered `busy`, naming it, with nothing
            // of it left. Any other failure may leave the transaction's end
            // unknown, and the record is kept for the same request to find
            // out.
            Err(error) => {
                let busy = error.kind == RepositoryErrorKind::RepositoryBusy;
                let blocker = error.blocker().cloned();
                Ran {
                    answer: refused(error).naming(service, &root, blocker.as_ref()),
                    standing: Standing::Stopped {
                        journal: busy.then_some(NO_ROW),
                    },
                }
            }
        }
    }

    /// The one domain call, and what it came to. An abandon is one
    /// transaction: it happened or it did not.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        if let Some(expected) = &self.remote {
            return self.call_remote(service, accepted, expected);
        }
        if let Some(metadata) = &self.retirement {
            let result = crate::repository::context_retirement::preserve::begin(
                service,
                &accepted.root(),
                self.operation_id,
                metadata,
                accepted,
            );
            let complete = service
                .retirement_mutation_state(&accepted.root(), self.operation_id)
                .ok()
                .flatten()
                .is_some_and(|s| s.phase == "abandoned");
            return match result {
                Ok(_) if complete => Ran {
                    answer: Answer {
                        effects: crate::results::Effects {
                            discovery: crate::results::DiscoveryEffect::Current,
                            ..crate::results::Effects::not_requested()
                        },
                        ..Answer::stopped(ResultCode::Ok)
                    }
                    .with_data(MutationDataDto::Operation(
                        OperationMutationDto {
                            operation_id: self.operation_id.to_string(),
                            action: Some(crate::repository::OperationAction::RetireContext),
                            state: "completed".into(),
                            step: Some("abandoned".into()),
                            item_id: None,
                            worktree: None,
                            paths: Vec::new(),
                            paths_complete: true,
                            committed: false,
                            released_request_id: None,
                        },
                    )),
                    standing: Standing::Final,
                },
                Ok(_) => Ran {
                    answer: Answer {
                        recovery: vec![preserving_resume(self.operation_id)],
                        effects: crate::results::Effects {
                            discovery: crate::results::DiscoveryEffect::Pending,
                            ..crate::results::Effects::not_requested()
                        },
                        ..Answer::stopped(ResultCode::DiscoveryPending)
                    },
                    standing: Standing::Owed,
                },
                Err(e) => Ran {
                    answer: refused(e),
                    standing: match crate::repository::context_retirement::preserve::owns(
                        service,
                        &accepted.root(),
                        self.operation_id,
                        accepted,
                    ) {
                        Ok(false) => Standing::Stopped {
                            journal: Some(NO_ROW),
                        },
                        _ => Standing::Owed,
                    },
                },
            };
        }
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
            // The ID is a synchronization's or a key operation's. No
            // preview is given of one, so this is not reached by a
            // request that was accepted on a preview; nothing was
            // changed.
            Ok(AbandonOperationOutcome::AnotherJournal) => Ran {
                answer: Answer::stopped(ResultCode::InvalidInput),
                standing: Standing::Stopped {
                    journal: Some(NO_ROW),
                },
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
            remote: _,
            retirement: _,
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
    ///
    /// An ID the key journal holds, or one of the remote journal that is
    /// no synchronization this command closes, is no state of a preview: no
    /// preview is given of one. It is refused here as the input it is. A
    /// synchronization another process is at work under is `busy`, naming
    /// it: nothing is recorded, and the same request goes on once it is
    /// done.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (scope, found) = self.observe(call.service)?;
        match &found {
            Found::Local(AbandonableOperation::AnotherJournal) => {
                return Err(Answer::stopped(ResultCode::InvalidInput));
            }
            Found::Remote(AbandonableRemote::Pending(_)) => {
                if let Some(answer) = self.held(call.service, &scope) {
                    return Err(answer);
                }
            }
            _ => {}
        }
        self.retirement = if matches!(found, Found::Local(AbandonableOperation::Pending(_))) {
            retirement_metadata(call.service, &scope, self.operation_id)?
        } else {
            None
        };
        let observation = Self::digest(&found, self.retirement.as_ref()).to_stored();
        match found {
            Found::Local(AbandonableOperation::Pending(remains)) => {
                self.seen = Some(remains.standing);
            }
            Found::Remote(AbandonableRemote::Pending(remains)) => {
                self.remote = Some(remains.standing);
            }
            _ => {}
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
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        confirm::reenter(self, call, accepted, journal)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        Some(self)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

impl Confirmed for OperationAbandonBinding {
    /// No row is read for a request with no operation. Nothing is run
    /// or settled on one that is.
    fn row(&mut self, call: &CallContext<'_>, accepted: &Accepted, journal: &JournalRow) -> Row {
        match crate::repository::context_retirement::preserve::owns(
            call.service,
            &accepted.root(),
            self.operation_id,
            accepted,
        ) {
            Ok(true) => {
                self.retirement = match crate::repository::context_retirement::preserve::snapshot(
                    call.service,
                    &accepted.root(),
                    self.operation_id,
                )
                .map_err(refused)
                {
                    Ok(metadata) => metadata,
                    Err(answer) => {
                        return Row::answers(Ran {
                            answer,
                            standing: Standing::Owed,
                        });
                    }
                };
                return Row::Unfinished;
            }
            Ok(false) => {}
            Err(error) => {
                return Row::answers(Ran {
                    answer: Answer::of_read(&error),
                    standing: Standing::Owed,
                });
            }
        }
        if *journal == NO_ROW {
            Row::Unrecorded
        } else {
            Row::Impossible
        }
    }

    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let (scope, found) = match self.observe(call.service) {
            Ok(observed) => observed,
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
        let found = match found {
            Found::Local(found) => found,
            // Closed, by this request's own work or another's: the end
            // state holds. One that completed is not what was shown.
            Found::Remote(AbandonableRemote::Ended { cancelled: true }) => {
                return Ok(Seen::EndState(Answer::stopped(ResultCode::AlreadyApplied)));
            }
            Found::Remote(
                AbandonableRemote::Ended { cancelled: false }
                | AbandonableRemote::NotFound
                | AbandonableRemote::Refused,
            ) => return Err(changed()),
            Found::Remote(AbandonableRemote::Pending(remains)) => {
                if let Some(answer) = self.held(call.service, &scope) {
                    return Err(Ran {
                        answer,
                        standing: Standing::NotRun { journal: NO_ROW },
                    });
                }
                self.remote = Some(remains.standing.clone());
                return Ok(Seen::State {
                    found: Self::digest(&Found::Remote(AbandonableRemote::Pending(remains)), None),
                    own: None,
                });
            }
        };
        match &found {
            AbandonableOperation::Abandoned => {
                Ok(Seen::EndState(Answer::stopped(ResultCode::AlreadyApplied)))
            }
            AbandonableOperation::Pending(remains) => {
                // What the domain call is told the row was seen as. It
                // is made only when the row is the one that was shown.
                self.seen = Some(remains.standing.clone());
                self.retirement = retirement_metadata(call.service, &scope, self.operation_id)
                    .map_err(|answer| Ran {
                        answer,
                        standing: Standing::Owed,
                    })?;
                Ok(Seen::State {
                    found: Self::digest(&Found::Local(found.clone()), self.retirement.as_ref()),
                    own: None,
                })
            }
            AbandonableOperation::AnotherJournal => Err(Ran {
                answer: Answer::stopped(ResultCode::InvalidInput),
                standing: Standing::NotRun { journal: NO_ROW },
            }),
            AbandonableOperation::NotFound
            | AbandonableOperation::Completed
            | AbandonableOperation::Rejected => Err(changed()),
        }
    }

    /// A preview is given of a pending operation of the local journal
    /// alone, and of a pending synchronization that
    /// `abandon_remote_operation` closes and no other process works under.
    /// An ID the key journal holds, or one of the remote journal that is no
    /// such synchronization, is `invalid_input`, asked first: this command
    /// does not abandon those. An ID no journal holds for the repository is
    /// `operation_not_found`, and an operation that has ended, however it
    /// ended, is a no-op.
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let (scope, found) = self.observe(call.service)?;
        let found = match found {
            Found::Local(found) => found,
            Found::Remote(remote) => return self.preview_remote(call, &scope, remote),
        };
        self.retirement = if matches!(found, AbandonableOperation::Pending(_)) {
            retirement_metadata(call.service, &scope, self.operation_id)?
        } else {
            None
        };
        if self
            .retirement
            .as_ref()
            .is_some_and(|s| s.phase == "preserving")
        {
            return Err(Answer {
                recovery: vec![preserving_resume(self.operation_id)],
                ..Answer::stopped(ResultCode::DiscoveryPending)
            });
        }
        let observation =
            Self::digest(&Found::Local(found.clone()), self.retirement.as_ref()).to_stored();
        let remains = match found {
            AbandonableOperation::Pending(remains) => remains,
            AbandonableOperation::NotFound => {
                return Err(Answer::stopped(ResultCode::OperationNotFound));
            }
            // A synchronization's or a key operation's, whatever the
            // local journal holds under the ID: the index hand-off of a
            // synchronization is a local row, and is the
            // synchronization's to end.
            AbandonableOperation::AnotherJournal => {
                return Err(Answer::stopped(ResultCode::InvalidInput));
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
            committed,
            owner,
        } = remains;
        let mut effects = vec![PreviewEffect::OperationClosed];
        if owner.is_some() && self.retirement.is_none() {
            effects.push(PreviewEffect::RequestReleased);
        }
        Ok(Previewed {
            observation,
            effects,
            details: PreviewDetailsDto::OperationAbandon(OperationAbandonPreviewDto {
                retirement: self.retirement.clone(),
                root: scope.as_str().to_owned(),
                operation_id: self.operation_id.to_string(),
                action: local_action(&action),
                state,
                step,
                item_id,
                worktree,
                paths,
                paths_complete,
                committed,
                request_id: owner,
                files_changed: false,
                git_state_changed: false,
                evidence: None,
            }),
        })
    }
}

impl OperationAbandonBinding {
    /// The preview of the abandon of a synchronization: what its row says
    /// of it, and what it last saw of the remote. One that has ended is a
    /// no-op, and one another process works under is `busy`.
    fn preview_remote(
        &mut self,
        call: &CallContext<'_>,
        scope: &ScopeKey,
        found: AbandonableRemote,
    ) -> Result<Previewed, Answer> {
        let remains = match found {
            AbandonableRemote::Pending(remains) => remains,
            AbandonableRemote::Ended { cancelled: _ } => {
                return Err(Answer::stopped(ResultCode::AlreadyApplied));
            }
            AbandonableRemote::NotFound | AbandonableRemote::Refused => {
                return Err(Answer::stopped(ResultCode::InvalidInput));
            }
        };
        if let Some(answer) = self.held(call.service, scope) {
            return Err(answer);
        }
        let observation = Self::digest(
            &Found::Remote(AbandonableRemote::Pending(remains.clone())),
            None,
        )
        .to_stored();
        let mut effects = vec![PreviewEffect::OperationClosed];
        if remains.answers_push {
            effects.push(PreviewEffect::PushMarkedRejected);
        }
        if remains.owner.is_some() {
            effects.push(PreviewEffect::RequestReleased);
        }
        let evidence = remains.evidence;
        Ok(Previewed {
            observation,
            effects,
            details: PreviewDetailsDto::OperationAbandon(OperationAbandonPreviewDto {
                retirement: None,
                root: scope.as_str().to_owned(),
                operation_id: self.operation_id.to_string(),
                action: Some(remote_action(remains.action)),
                state: remote_phase(remains.standing.phase).0.to_owned(),
                step: remains
                    .standing
                    .step
                    .map(|step| remote_step(step).to_owned()),
                item_id: remains.item_id.clone(),
                worktree: None,
                paths: Vec::new(),
                paths_complete: true,
                committed: false,
                request_id: remains.owner.clone(),
                files_changed: false,
                git_state_changed: false,
                evidence: Some(OperationAbandonEvidenceDto {
                    remote_ref_present: evidence.remote_ref_present,
                    local_oid: evidence.local_oid.map(|oid| oid.to_string()),
                    remote_oid: evidence.remote_oid.map(|oid| oid.to_string()),
                    push_oid: evidence.push_oid.map(|oid| oid.to_string()),
                }),
            }),
        })
    }
}

fn retirement_metadata(
    service: &RepositoryService,
    scope: &ScopeKey,
    operation: OperationId,
) -> Result<Option<crate::repository::RetirementAbandonMetadata>, Answer> {
    service
        .read_session(crate::repository::RepositoryOperation::Read, |db| {
            crate::repository::context_retirement::preserve::metadata(
                db,
                Path::new(scope.as_str()),
                operation,
            )
            .map_err(|_| crate::repository::ReadError::new(ResultCode::RecoveryRequired))
        })
        .map_err(|e| Answer::of_read(&e))
}
fn preserving_resume(operation: OperationId) -> crate::results::RecoveryAction {
    crate::results::RecoveryAction::for_operation(
        crate::results::RecoveryActionKind::OperationResume,
        operation.to_string(),
        [],
    )
}
