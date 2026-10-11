//! `repo remove`: a repository is taken out of the repository list
//! again. It needs confirmation.

use std::path::PathBuf;

use crate::results::{ResultCode, Scope, timestamp_string};

use super::{
    super::{
        super::{
            OperationFamily, RemoveRegistrationRequest, RepositoryOperation, RepositoryService,
            canonical_repository_root,
            recovery::{FinalKind, JournalRow, PendingOperation},
        },
        command::Command,
        dto::{PreviewDetailsDto, PreviewEffect, RepositoryRemovalPreviewDto},
        evidence::Position,
        identity::IntentDigestBuilder,
        observe::{Observation, ObservationDigest},
        outcome,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused, resolve_root,
};

/// The input of `repo remove`: the root of a registered repository.
#[derive(Clone)]
pub struct RepoRemoveInput {
    pub root: PathBuf,
}

/// `repo remove`: the repository is taken out of the repository list,
/// and what Manyhands cached of it is dropped with it. It needs
/// confirmation. No file under the root and nothing Git holds is
/// touched, so no effect shows it and it carries no data: the outcome
/// says whether anything changed.
///
/// **What its preview covers.** The registration: that the root is
/// registered, and which registration it is. The command reads no
/// identity.
///
/// **Its operation's row.** `remove_registration` records no step, and
/// deletes every row of the repository's local journal, its own among
/// them, in the transaction that removes the registration. So a request
/// that completed has no row, and what shows its work to be done is that
/// the root is not registered. A row that is there has done nothing.
pub(crate) struct RepoRemoveBinding {
    root: PathBuf,
}

impl From<RepoRemoveInput> for RepoRemoveBinding {
    fn from(input: RepoRemoveInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RepoRemoveInput { root } = input;
        Self { root }
    }
}

/// What a preview of `repo remove` covers.
struct Registered {
    /// The repository's root, as a registration of it holds it.
    root: PathBuf,
    /// The registration's ID and the time it was made. `None` when the
    /// root is not registered.
    registration: Option<(i64, i64)>,
}

impl RepoRemoveBinding {
    /// Reads what a preview covers. Nothing is written and no lease is
    /// taken. An `Err` is a root that is no repository's, or a read that
    /// failed. A repository that is not registered is no error here: it
    /// is what the command leaves.
    fn observe(&self, service: &RepositoryService) -> Result<Registered, Answer> {
        let root = match resolve_root(service, &self.root) {
            Ok(resolved) => resolved.root().to_owned(),
            Err(answer) if answer.code == ResultCode::RepositoryNotRegistered => {
                canonical_repository_root(&self.root, RepositoryOperation::RemoveRegistration)
                    .map_err(refused)?
                    .1
            }
            Err(answer) => return Err(answer),
        };
        let registration = service
            .registration(&root)
            .map_err(|error| Answer::of_read(&error))?;
        Ok(Registered { root, registration })
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with.
    fn digest(registered: &Registered) -> ObservationDigest {
        let Registered {
            // The request's scope.
            root: _,
            registration,
        } = registered;
        let command = Command::RepoRemove.name();
        let observation = Observation::of(command);
        let observation = match registration {
            Some((id, enabled_at)) => observation
                .text("registration", &id.to_string())
                .text("enabled_at", &enabled_at.to_string()),
            None => observation.absent("registration").absent("enabled_at"),
        };
        ObservationDigest::of_state(command, observation.finish())
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to. A removal is one transaction: it happened or it did not,
    /// and nothing of it is ever in flight but its row.
    fn call(service: &RepositoryService, accepted: &Accepted) -> Ran {
        let result = service.remove_registration(RemoveRegistrationRequest {
            root: accepted.root(),
            operation_id: accepted.operation_id,
        });
        match result {
            Ok(outcome) => Ran {
                answer: Answer::stopped(outcome::remove_registration_outcome_code(&outcome)),
                standing: Standing::Final,
            },
            // Only the kind is read.
            Err(error) => Ran {
                answer: refused(error),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            },
        }
    }
}

impl Binding for RepoRemoveBinding {
    fn command(&self) -> Command {
        Command::RepoRemove
    }

    /// `remove_registration` records no step and completes no row: the
    /// row of a removal that happened is deleted with the registration.
    /// No row says of this command that nothing was done.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    /// The repository the request removes the registration of. Its
    /// scope key is made from the root on disk, which the command leaves
    /// as it is: a request that removed the registration is still found
    /// under it.
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

    /// Observes. No check is made of what the preview covers: the
    /// boundary compares the digest with the confirmation's, and refuses
    /// a request whose preview is stale as `external_change`.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let registered = self.observe(call.service)?;
        Ok(Prepared {
            family: Some(OperationFamily::Local),
            // Nothing is committed, so there is no branch to bound.
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
            observation: Some(Self::digest(&registered).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        Self::call(call.service, accepted)
    }

    /// The registration is asked first, whatever the row says:
    ///
    /// 1. The root is not registered. The request's end state holds, by
    ///    its own work or another's: a removal leaves no row to tell the
    ///    two apart. With no row the request is a no-op and is finished.
    ///    A row that is pending is the domain's to end, and it is called.
    /// 2. The root is registered, and the registration is the one that
    ///    was previewed: nothing of the request's has been done, and the
    ///    domain is called.
    /// 3. The root is registered, by another registration than the one
    ///    previewed: the root was registered again. The previewed one is
    ///    gone, which is the request's end state; with no row the
    ///    request is a no-op and is finished, and the new registration is
    ///    not removed on a consent to the first. A completed removal
    ///    whose result was lost is answered so, and never as
    ///    `external_change`.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        let not_run = |answer: Answer, journal: JournalRow| Ran {
            answer,
            standing: Standing::NotRun { journal },
        };
        match &journal {
            JournalRow::Absent
            | JournalRow::Pending(PendingOperation::Local { state: _, step: _ }) => {}
            // The operation is a local one, and its row never completes:
            // it is deleted with the registration, and one that was
            // rejected reads as absent. Nothing is run or settled on a
            // row that says otherwise.
            JournalRow::Pending(
                PendingOperation::Remote { phase: _ } | PendingOperation::KeyMaterial { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => {
                return Err(not_run(Answer::stopped(ResultCode::InternalError), journal));
            }
        }
        let registered = match self.observe(call.service) {
            Ok(registered) => registered,
            Err(answer) => return Err(not_run(answer, journal)),
        };
        if registered.registration.is_none() {
            return if journal == JournalRow::Absent {
                Err(Ran {
                    answer: Answer::stopped(ResultCode::AlreadyApplied),
                    standing: Standing::Final,
                })
            } else {
                Ok(Entered::Unfinished)
            };
        }
        // A request whose record holds no consent that can be read has
        // none that anything could be compared with: nothing is run.
        let Some(previewed) = call.previewed() else {
            return Err(not_run(
                Answer::stopped(ResultCode::ExternalChange),
                journal,
            ));
        };
        if previewed == Self::digest(&registered) {
            return Ok(Entered::Unfinished);
        }
        // Another registration than the one that was previewed. The
        // request was accepted with the previewed one in place, so that
        // one has been removed since, by this request or by another: a
        // removal leaves no row to say which. With no row the request's
        // end state holds, and the registration that is there now is not
        // the request's to remove. A row that is pending has removed
        // nothing, and refuses everything else on the repository: what
        // is there is not what was shown.
        if journal == JournalRow::Absent {
            Err(Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            })
        } else {
            Err(not_run(
                Answer::stopped(ResultCode::ExternalChange),
                journal,
            ))
        }
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        Self::call(call.service, accepted)
    }
}

impl Confirmed for RepoRemoveBinding {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let registered = self.observe(call.service)?;
        let Some((_, enabled_at)) = registered.registration else {
            // The command's end state holds.
            return Err(Answer::stopped(ResultCode::AlreadyApplied));
        };
        Ok(Previewed {
            observation: Self::digest(&registered).to_stored(),
            effects: vec![
                PreviewEffect::RegistrationRemoved,
                PreviewEffect::IndexDropped,
                PreviewEffect::OperationRecordsDropped,
                PreviewEffect::RemoteStateDropped,
            ],
            details: PreviewDetailsDto::RepositoryRemoval(RepositoryRemovalPreviewDto {
                root: registered.root.to_string_lossy().into_owned(),
                enabled_at: time::OffsetDateTime::from_unix_timestamp(enabled_at)
                    .ok()
                    .and_then(timestamp_string),
                files_changed: false,
                git_state_changed: false,
            }),
        })
    }
}
