//! The shared-key commands. They belong to the application and to no
//! repository: a request of one is recorded under the scope `application`,
//! and its results name no repository, item, branch or worktree.
//!
//! A key is no canonical content and no commit, so no effect shows one:
//! every effect is `not_requested`, `data` names the key, and the outcome
//! says whether anything changed.
//!
//! `key select`, `key clear` and `key remove` change the key registry
//! and nothing else. The domain call of each is one transaction with no
//! operation ID and no journal row: it happened or it did not. A request
//! of one is therefore accepted with no operation, nothing is ever in
//! flight under it, and it is settled as any request with no journal row
//! is: finished when its end state holds, deleted when it was refused.

use crate::results::{ResultCode, Scope};

use super::{
    super::{
        super::{ReadError, RepositoryService, SharedKeyId, SharedKeySelectionOutcome},
        command::Command,
        dto::{KeyMutationDto, MutationDataDto},
        evidence::Position,
        identity::IntentDigestBuilder,
        outcome,
    },
    Accepted, Answer, Binding, CallContext, EndedWithoutCheckpoint, Entered, Place, Prepared, Ran,
    Standing,
};
use crate::repository::{SharedKeyRegistration, recovery::JournalRow};

/// The input of `key select`: the registered key to authenticate with.
pub struct KeySelectInput {
    pub id: SharedKeyId,
}

/// The input of `key clear`, which takes none: no key is selected
/// afterwards.
pub struct KeyClearInput {}

/// The input of `key remove`: the key whose registration is removed. Its
/// files are left where they are.
pub struct KeyRemoveInput {
    pub id: SharedKeyId,
}

/// The position of a request that commits nothing: there is no branch to
/// bound.
fn no_position() -> Position {
    Position {
        base_ref: None,
        base_oid: None,
    }
}

/// What a key command reports of the key `id`, with what was read of its
/// registration.
fn key_data(
    id: Option<SharedKeyId>,
    label: Option<&str>,
    fingerprint: Option<&str>,
) -> MutationDataDto {
    MutationDataDto::Key(KeyMutationDto {
        id: id.map(|id| id.to_string()),
        label: label.map(str::to_owned),
        fingerprint: fingerprint.map(str::to_owned),
        generation: None,
    })
}

/// The data of a registration the domain returned.
fn registered(registration: &SharedKeyRegistration) -> MutationDataDto {
    key_data(
        Some(registration.id),
        Some(&registration.label),
        registration.public_key_fingerprint.as_deref(),
    )
}

/// What a change to the key registry asks for.
enum Change {
    Select(SharedKeyId),
    Clear,
    Remove(SharedKeyId),
}

/// `key select`, `key clear` and `key remove`: one binding, since each is
/// one registry transaction with no operation.
pub(crate) struct KeyRegistryBinding {
    change: Change,
}

impl From<KeySelectInput> for KeyRegistryBinding {
    fn from(input: KeySelectInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let KeySelectInput { id } = input;
        Self {
            change: Change::Select(id),
        }
    }
}

impl From<KeyClearInput> for KeyRegistryBinding {
    fn from(input: KeyClearInput) -> Self {
        let KeyClearInput {} = input;
        Self {
            change: Change::Clear,
        }
    }
}

impl From<KeyRemoveInput> for KeyRegistryBinding {
    fn from(input: KeyRemoveInput) -> Self {
        let KeyRemoveInput { id } = input;
        Self {
            change: Change::Remove(id),
        }
    }
}

impl KeyRegistryBinding {
    /// The key the request names, if it names one.
    fn id(&self) -> Option<SharedKeyId> {
        match self.change {
            Change::Select(id) | Change::Remove(id) => Some(id),
            Change::Clear => None,
        }
    }

    /// The request's data when nothing was read of its key.
    fn named(&self) -> MutationDataDto {
        key_data(self.id(), None, None)
    }

    /// A read of the registry failed, or found no such key: the read's
    /// code and recovery, naming the key.
    fn unread(&self, error: &ReadError) -> Answer {
        Answer::of_read(error).with_data(self.named())
    }

    /// The one domain call and what it came to.
    ///
    /// Each of these is also the evidence that the request's work is
    /// done, for a call that enters the request again: the registry is
    /// read, or changed, in the state it is in now, and a state that is
    /// already as requested is a no-op.
    fn change(&self, service: &RepositoryService) -> Answer {
        let answer = |code, data| Answer::stopped(code).with_data(data);
        // Only the kind of a domain error is read.
        let failed = |kind| answer(outcome::repository_error_code(kind), self.named());
        match self.change {
            // The domain selects a key that is already selected and says
            // the same of both, so the registry is read first. A key
            // that already is selected is left as it is.
            Change::Select(id) => match service.show_key(id) {
                Ok(key) if key.selected => answer(
                    ResultCode::AlreadyApplied,
                    key_data(Some(id), Some(&key.label), key.fingerprint.as_deref()),
                ),
                Ok(_) => match service.select_shared_key(id) {
                    Ok(selection) => {
                        let code = outcome::key_selection_outcome_code(&selection);
                        match selection {
                            SharedKeySelectionOutcome::Selected(registration) => {
                                answer(code, registered(&registration))
                            }
                            // Not what a selection returns.
                            SharedKeySelectionOutcome::Cleared
                            | SharedKeySelectionOutcome::AlreadyCleared => {
                                answer(ResultCode::InternalError, self.named())
                            }
                        }
                    }
                    Err(error) => failed(error.kind),
                },
                Err(error) => self.unread(&error),
            },
            Change::Clear => match service.clear_shared_key_selection() {
                Ok(selection) => answer(
                    outcome::key_selection_outcome_code(&selection),
                    self.named(),
                ),
                Err(error) => failed(error.kind),
            },
            Change::Remove(id) => match service.unregister_shared_key(id) {
                Ok(removal) => answer(outcome::unregister_key_outcome_code(removal), self.named()),
                Err(error) => failed(error.kind),
            },
        }
    }

    /// The domain call, settled: a code with no failure class is the
    /// request's end state, and anything else stopped it with nothing in
    /// flight, since nothing of a registry change is ever in flight.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let answer = self.change(service);
        let standing = match answer.code.failure_class() {
            None => Standing::Final,
            Some(_) => Standing::Stopped {
                journal: accepted.journal_row(service),
            },
        };
        Ran { answer, standing }
    }
}

impl Binding for KeyRegistryBinding {
    fn command(&self) -> Command {
        match self.change {
            Change::Select(_) => Command::KeySelect,
            Change::Clear => Command::KeyClear,
            Change::Remove(_) => Command::KeyRemove,
        }
    }

    /// A registry change has no operation, so no row of one is ever read
    /// as final. It answers as every command that is no authoring write
    /// does: no row says of it that nothing was done.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Application
    }

    /// The key, or nothing for `key clear`, which names none.
    fn target(&self) -> String {
        self.id().map(|id| id.to_string()).unwrap_or_default()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        // The command and the target are what the digest is begun with,
        // and no change holds anything else.
        match self.change {
            Change::Select(_) | Change::Clear | Change::Remove(_) => builder,
        }
    }

    /// A key command names no repository, item, branch or worktree.
    fn scope(&self, _repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope::default()
    }

    /// A key that is not registered is refused here, before the request
    /// is accepted. `key remove` depends on it: the domain reports a key
    /// that is not registered as one already removed, which is the
    /// answer for a request that was accepted while the key was there
    /// and not for one that names a key the registry never held.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        match self.change {
            Change::Select(id) | Change::Remove(id) => {
                call.service
                    .show_key(id)
                    .map_err(|error| self.unread(&error))?;
            }
            Change::Clear => {}
        }
        Ok(Prepared {
            family: None,
            position: no_position(),
            expected: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// The request has no operation, so its row is absent whatever an
    /// earlier call did, and the domain call is made again: it is its own
    /// evidence of what is done, as `change` has it.
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match journal {
            JournalRow::Absent => Ok(Entered::Unfinished),
            // No row is read for a request with no operation. Nothing is
            // run or settled on one that is.
            JournalRow::Pending(_)
            | JournalRow::Final {
                kind: _,
                owes_work: _,
                checkpointed: _,
            } => Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            }),
        }
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}
