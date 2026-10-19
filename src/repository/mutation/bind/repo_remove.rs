//! `repo remove`: a repository is taken out of the repository list
//! again. It needs confirmation.

use crate::repository::mutation::confirm::{self, Row, Seen};
use std::path::PathBuf;

use crate::results::{ResultCode, Scope, timestamp_string};

use super::{
    super::{
        super::{
            OperationFamily, RepositoryErrorKind, RepositoryOperation, RepositoryService,
            canonical_repository_root,
            recovery::{FinalKind, JournalRow, PendingOperation},
            registration_removal::{RegistrationStamp, RemovalTarget, repository_at},
        },
        command::Command,
        dto::{PreviewDetailsDto, PreviewEffect, RepositoryRemovalPreviewDto},
        evidence::Position,
        identity::{IntentDigestBuilder, ScopeKey},
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
    selected_root: Option<PathBuf>,
    observed: Option<RemovalTarget>,
}

impl From<RepoRemoveInput> for RepoRemoveBinding {
    fn from(input: RepoRemoveInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RepoRemoveInput { root } = input;
        Self {
            root,
            selected_root: None,
            observed: None,
        }
    }
}

impl RepoRemoveBinding {
    /// Reads the registration without a lease. Fresh selection validates the
    /// path, allowing a registered root to be absent. A consent/request-bound
    /// observation reads only the registry: its absent row is the end state,
    /// while an unknown missing path is still an error for fresh selection.
    fn observe(
        &mut self,
        service: &RepositoryService,
        bound: bool,
    ) -> Result<RemovalTarget, Answer> {
        self.observed = None;
        let selected = self.selected_root.as_ref().unwrap_or(&self.root);
        let registration = service
            .registration(selected)
            .map_err(|error| Answer::of_read(&error))?;
        let root = if bound {
            selected.clone()
        } else if registration.is_some() {
            // Preview/first selection must distinguish absence from damaged or
            // inaccessible Git. A bound re-entry observes only its registration.
            repository_at(selected).map_err(refused)?;
            selected.clone()
        } else {
            match resolve_root(service, selected) {
                Ok(resolved) => resolved.root().to_owned(),
                // Whether it was ever enabled is not asked: the end state
                // is the same.
                Err(answer)
                    if answer.code == ResultCode::RepositoryNotRegistered
                        || answer.code == ResultCode::RepositoryNotEnabled =>
                {
                    canonical_repository_root(selected, RepositoryOperation::RemoveRegistration)
                        .map_err(refused)?
                        .1
                }
                Err(answer) => return Err(answer),
            }
        };
        let registration = service
            .registration(&root)
            .map_err(|error| Answer::of_read(&error))?;
        let found = RemovalTarget {
            root,
            registration: registration.map(|(id, enabled_at)| RegistrationStamp { id, enabled_at }),
        };
        self.observed = Some(found.clone());
        Ok(found)
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with.
    fn digest(registered: &RemovalTarget) -> ObservationDigest {
        let RemovalTarget {
            // The request's scope.
            root: _,
            registration,
        } = registered;
        let command = Command::RepoRemove.name();
        let observation = Observation::of(command);
        let observation = match registration {
            Some(RegistrationStamp { id, enabled_at }) => observation
                .text("registration", &id.to_string())
                .text("enabled_at", &enabled_at.to_string()),
            None => observation.absent("registration").absent("enabled_at"),
        };
        ObservationDigest::of_state(command, observation.finish())
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to. A removal is one transaction: it happened or it did not,
    /// and nothing of it is ever in flight but its row.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let Some(target) = self
            .observed
            .as_ref()
            .filter(|target| target.root == accepted.root())
        else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            };
        };
        let result = service.remove_registration_checked(target.clone(), accepted.operation_id);
        match result {
            Ok(outcome) => Ran {
                answer: Answer::stopped(outcome::remove_registration_outcome_code(&outcome)),
                standing: Standing::Final,
            },
            Err(error) if error.kind == RepositoryErrorKind::ExternalChange => {
                let journal = accepted.journal_row(service);
                // Another attempt can remove the consented registration after
                // this retry observed it. Successful removal deletes its own
                // journal, so absence is the accepted request's end state, not
                // proof that the request never ran. Keep a durable no-op receipt
                // and leave any replacement registration alone. A fresh call
                // still refuses a registration changed before its first effect.
                if accepted.attempt > 1 && journal == Some(JournalRow::Absent) {
                    return Ran {
                        answer: Answer::stopped(ResultCode::AlreadyApplied),
                        standing: Standing::Final,
                    };
                }
                let standing = match journal {
                    Some(journal) => Standing::NotRun { journal },
                    None => Standing::Stopped { journal: None },
                };
                Ran {
                    answer: refused(error),
                    standing,
                }
            }
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

    /// The original selection, used to place failures before resolution.
    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    fn resolve_scope(
        &mut self,
        service: &RepositoryService,
        persisted: Option<&ScopeKey>,
    ) -> Result<Option<ScopeKey>, Answer> {
        self.selected_root = None;
        self.observed = None;
        let exact_stored = self.root.is_absolute()
            && persisted
                .and_then(ScopeKey::repository)
                .is_some_and(|root| self.root.to_str() == Some(root));
        let key = if exact_stored {
            // Command, target, digest and consent are checked by the boundary.
            persisted.cloned()
        } else if self.root.is_absolute()
            && service
                .registration(&self.root)
                .map_err(|error| Answer::of_read(&error))?
                .is_some()
        {
            self.root.to_str().map(ScopeKey::from_stored)
        } else {
            // A live alias can resolve. An unavailable alias cannot be guessed.
            // Detailed invalid-path/Git errors belong to observation, not identity.
            ScopeKey::for_repository(&self.root)
        };
        self.selected_root = key
            .as_ref()
            .and_then(ScopeKey::repository)
            .map(PathBuf::from);
        Ok(key)
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
            selected_root: _,
            observed: _,
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
        let registered = self.observe(call.service, call.confirmation.is_some())?;
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
        self.call(call.service, accepted)
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

impl Confirmed for RepoRemoveBinding {
    fn row(&mut self, _call: &CallContext<'_>, _accepted: &Accepted, journal: &JournalRow) -> Row {
        match journal {
            JournalRow::Absent
            | JournalRow::Pending(PendingOperation::Local { state: _, step: _ }) => Row::Unrecorded,
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
            } => Row::Impossible,
        }
    }

    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        self.selected_root = Some(accepted.root());
        let registered = self.observe(call.service, true).map_err(|answer| Ran {
            answer,
            standing: Standing::NotRun {
                journal: journal.clone(),
            },
        })?;
        Ok(if registered.registration.is_none() {
            Seen::EndState(Answer::stopped(ResultCode::AlreadyApplied))
        } else {
            Seen::State {
                found: Self::digest(&registered),
                own: None,
            }
        })
    }

    /// Another registration than the one that was previewed. The
    /// request was accepted with the previewed one in place, so that
    /// one has been removed since, by this request or by another: a
    /// removal leaves no row to say which. With no row the request's
    /// end state holds, and the registration that is there now is not
    /// the request's to remove. A row that is pending has removed
    /// nothing, and refuses everything else on the repository: what
    /// is there is not what was shown.
    ///
    /// A request whose record holds no consent that can be read has
    /// none that anything could be compared with: nothing is run.
    fn differs(&mut self, journal: JournalRow, consented: bool) -> Ran {
        if consented && journal == JournalRow::Absent {
            Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            }
        } else {
            Ran {
                answer: Answer::stopped(ResultCode::ExternalChange),
                standing: Standing::NotRun { journal },
            }
        }
    }

    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let registered = self.observe(call.service, false)?;
        let Some(RegistrationStamp { enabled_at, .. }) = registered.registration else {
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
