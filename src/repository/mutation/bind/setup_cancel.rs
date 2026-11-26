//! `repo setup-cancel`: a pending setup is cancelled (design Part H). It
//! needs confirmation.
//!
//! The setup's remote branch is deleted only at its recorded commit, the
//! local branch, the private base ref and the record are removed, a
//! first-time setup is unregistered, and the setup's request and this one
//! are finished in the transaction that ends the setup, so a request sent
//! again after any crash answers the stored result.

use std::path::{Path, PathBuf};

use crate::{
    repository::{
        OperationFamily, RepositoryOperation, RepositoryService, canonical_repository_root,
        mutation::{
            command::Command,
            confirm::{self, Row, Seen},
            dto::{
                MutationDataDto, PreviewDetailsDto, PreviewEffect, SetupCancelDto,
                SetupCancelPreviewDto,
            },
            evidence::Position,
            identity::IntentDigestBuilder,
            observe::{Observation, ObservationDigest},
            outcome,
        },
        recovery::{FinalKind, JournalRow, LocalStep, PendingOperation},
        setup::{
            cancel::{CancelSetupRequest, SetupCancelError, current_setup, state_key},
            record::{SetupKind, SetupRecord},
        },
        transport::SshTransportError,
    },
    results::{ResultCode, Scope},
};

use super::{
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused,
};

/// The input of `repo setup-cancel`: the root of a registered repository.
#[derive(Clone)]
pub struct RepoSetupCancelInput {
    pub root: PathBuf,
}

/// `repo setup-cancel`.
///
/// **What its preview covers.** The setup the registry holds for the root:
/// its ID, kind, state, branch and remote. It reads the registry alone and
/// connects to nothing. A publish between the preview and the confirm changes
/// the state the digest binds, and the confirm is `external_change`.
///
/// **Its operation's row.** The cancel's own local row, `cancel_setup`, whose
/// target is the setup's ID and whose step `setup_cancel_recorded` is its
/// checkpoint: a request sent again goes on from the record, and the domain
/// does every step again, each being idempotent.
pub(crate) struct RepoSetupCancelBinding {
    root: PathBuf,
}

impl From<RepoSetupCancelInput> for RepoSetupCancelBinding {
    fn from(input: RepoSetupCancelInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RepoSetupCancelInput { root } = input;
        Self { root }
    }
}

/// The digest of the state a preview shows and a confirm is compared with.
fn digest(record: Option<&SetupRecord>) -> ObservationDigest {
    let command = Command::RepoSetupCancel.name();
    ObservationDigest::of_state(
        command,
        Observation::of(command)
            .text("state", &state_key(record))
            .finish(),
    )
}

/// The setup of the repository registered at `root`: none for a root that is
/// not registered or has none, and an answer for a registry that cannot be
/// read.
fn setup_of(service: &RepositoryService, root: &Path) -> Result<Option<SetupRecord>, Answer> {
    current_setup(service, root).map_err(|error| Answer::stopped(error.code()))
}

/// The host a transport stop is about, as a recovery action names it:
/// `host:port`, with an address that holds a colon in brackets.
fn host_of(error: &SshTransportError) -> Option<String> {
    let authority = error.authority.as_ref()?;
    Some(if authority.host.contains(':') {
        format!("[{}]:{}", authority.host, authority.port)
    } else {
        format!("{}:{}", authority.host, authority.port)
    })
}

/// Whether the request's own row is at `setup_cancel_recorded`, or cannot be
/// read: the request is then owed.
fn recorded_now(service: &RepositoryService, accepted: &Accepted) -> bool {
    match accepted.recorded_local_step(service) {
        Ok(step) => step.is_some_and(|step| step.is(LocalStep::SetupCancelRecorded)),
        Err(()) => true,
    }
}

impl RepoSetupCancelBinding {
    /// The setup the call is about, from the registry alone. No setup is the
    /// command's end state.
    fn observe(&self, service: &RepositoryService) -> Result<SetupRecord, Answer> {
        let (_, root) = canonical_repository_root(&self.root, RepositoryOperation::CancelSetup)
            .map_err(refused)?;
        if service
            .registration(&root)
            .map_err(|error| Answer::of_read(&error))?
            .is_none()
        {
            return Err(Answer::stopped(ResultCode::RepositoryNotRegistered));
        }
        setup_of(service, &root)?.ok_or_else(|| Answer::stopped(ResultCode::AlreadyApplied))
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let service = call.service;
        let root = accepted.root();
        let request = CancelSetupRequest {
            root: root.clone(),
            operation_id: accepted.operation_id,
            request_id: accepted.request_id,
        };
        let Some(result) = call.session.cancel_setup(service, &request) else {
            // No credentials: nothing was called. Owed once the row is
            // recorded, and settled as a refusal before.
            return Self::stop(
                service,
                accepted,
                Answer::stopped(ResultCode::InternalError),
            );
        };
        match result {
            Ok(report) => Ran {
                answer: Answer {
                    data: Some(MutationDataDto::SetupCancel(Box::new(SetupCancelDto::of(
                        &report,
                    )))),
                    local_change: true,
                    ..Answer::stopped(ResultCode::Ok)
                },
                // Finished by the domain, in the transaction that ended the
                // setup: `settled` answers the stored envelope.
                standing: Standing::Final,
            },
            // The setup is not there: the end state holds.
            Err(SetupCancelError::NoSetup) => Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            },
            Err(SetupCancelError::Ended(stored)) => Ran {
                answer: Answer::stopped(stored.code),
                standing: Standing::Final,
            },
            Err(error) => {
                let code = error.code();
                let mut answer = Answer::stopped(code);
                match &error {
                    SetupCancelError::InTheWay(blocker) => {
                        answer = answer.naming(service, &root, blocker.as_ref());
                    }
                    SetupCancelError::Transport(transport) => {
                        if let (Some(authority), Some(root)) = (host_of(transport), root.to_str()) {
                            answer.recovery =
                                outcome::host_action(code, root, &authority, accepted.request_id);
                        }
                    }
                    _ => {}
                }
                Self::stop(service, accepted, answer)
            }
        }
    }

    /// A stop: owed once the request's own row is recorded, so that the same
    /// request goes on; settled as a refusal before it, as any refusal of a
    /// first call is.
    fn stop(service: &RepositoryService, accepted: &Accepted, answer: Answer) -> Ran {
        let standing = if recorded_now(service, accepted) {
            Standing::Owed
        } else {
            Standing::Stopped {
                journal: accepted.journal_row(service),
            }
        };
        Ran { answer, standing }
    }
}

impl Binding for RepoSetupCancelBinding {
    fn command(&self) -> Command {
        Command::RepoSetupCancel
    }

    /// The row completes at `setup_cancel_recorded`, a checkpoint step, so
    /// this is not asked of a row that did its work; a completed row with no
    /// step is not one this command makes.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The repository as a whole: the command names nothing within it.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with. The request holds nothing
            // else.
            root: _,
        } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Observes. No check is made of what the preview covers: the boundary
    /// compares the digest with the confirmation's, and refuses a request
    /// whose preview is stale as `external_change`. A repository with no
    /// setup is the command's end state.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let record = self.observe(call.service)?;
        Ok(Prepared {
            family: Some(OperationFamily::Local),
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
            observation: Some(digest(Some(&record)).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted)
    }

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
        self.call(call, accepted)
    }
}

impl Confirmed for RepoSetupCancelBinding {
    /// How the request's own row reads:
    ///
    /// - Absent, or begun and not completed: nothing was recorded; the state
    ///   is observed again and compared with what was previewed.
    /// - Completed at `setup_cancel_recorded` with the setup still there: the
    ///   cancel recorded its row and is owed; the domain goes on, comparing
    ///   nothing, because its own work changed what would be observed.
    /// - Completed at `setup_cancel_recorded` and the setup gone: it ended
    ///   elsewhere, since the transaction that ends it finishes this request,
    ///   which would have been answered from its stored result: the end state
    ///   holds. A registry that cannot be read is owed.
    fn row(&mut self, call: &CallContext<'_>, accepted: &Accepted, journal: &JournalRow) -> Row {
        match journal {
            JournalRow::Absent | JournalRow::Pending(PendingOperation::Local { .. }) => {
                Row::Unrecorded
            }
            JournalRow::Final {
                kind: FinalKind::Completed,
                ..
            } => match accepted.recorded_local_step(call.service) {
                Ok(Some(step)) if step.is(LocalStep::SetupCancelRecorded) => {
                    match setup_of(call.service, &accepted.root()) {
                        Ok(Some(_)) => Row::Unfinished,
                        Ok(None) => Row::answers(Ran {
                            answer: Answer::stopped(ResultCode::AlreadyApplied),
                            standing: Standing::Final,
                        }),
                        Err(answer) => Row::answers(Ran {
                            answer,
                            standing: Standing::Owed,
                        }),
                    }
                }
                Ok(_) => Row::Impossible,
                Err(()) => Row::answers(Ran {
                    answer: Answer::stopped(ResultCode::RecoveryRequired),
                    standing: Standing::Owed,
                }),
            },
            JournalRow::Pending(
                PendingOperation::Remote { .. } | PendingOperation::KeyMaterial { .. },
            )
            | JournalRow::Final {
                kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                ..
            } => Row::Impossible,
        }
    }

    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        match setup_of(call.service, &accepted.root()) {
            Ok(None) => Ok(Seen::EndState(Answer::stopped(ResultCode::AlreadyApplied))),
            Ok(Some(record)) => Ok(Seen::State {
                found: digest(Some(&record)),
                own: None,
            }),
            Err(answer) => Err(Ran {
                answer,
                standing: Standing::NotRun {
                    journal: journal.clone(),
                },
            }),
        }
    }

    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let record = self.observe(call.service)?;
        let mut effects = Vec::new();
        if record.commit_oid.is_some() {
            effects.push(PreviewEffect::SetupBranchDeletedRemotely);
        }
        effects.push(PreviewEffect::SetupBranchDeleted);
        effects.push(PreviewEffect::SetupRecordRemoved);
        if record.kind == SetupKind::Enable {
            effects.push(PreviewEffect::RepositoryUnregistered);
        }
        let (_, root) = canonical_repository_root(&self.root, RepositoryOperation::CancelSetup)
            .map_err(refused)?;
        Ok(Previewed {
            observation: digest(Some(&record)).to_stored(),
            effects,
            details: PreviewDetailsDto::SetupCancel(SetupCancelPreviewDto {
                root: root.to_string_lossy().into_owned(),
                setup_id: record.setup_id.to_string(),
                kind: record.kind.as_str().to_owned(),
                state: record.state().as_str().to_owned(),
                branch: format!("manyhands/setup/{}", record.setup_id),
                remote: record.remote_name.clone(),
            }),
        })
    }
}
