//! The shared-key commands. They belong to the application and to no
//! repository: a request of one is recorded under the scope `application`,
//! and its results name no repository, item, branch or worktree.
//!
//! A key is no canonical content and no commit, so no effect shows one:
//! every effect is `not_requested`, `data` names the key, and the outcome
//! says whether anything changed.
//!
//! `key import`, `key select`, `key clear` and `key remove` change the
//! key registry and nothing else. The domain call of each is one transaction with no
//! operation ID and no journal row: it happened or it did not. A request
//! of one is therefore accepted with no operation, nothing is ever in
//! flight under it, and it is settled as any request with no journal row
//! is: finished when its end state holds, deleted when it was refused.
//!
//! `key generate` writes a key pair into the user's key store and
//! registers it, as one operation in the key journal. That journal is
//! not the local one. Its rows are of one key each: a generation that has
//! not ended blocks no other key operation and no repository, so its
//! record waits on nothing but its own retry. A row that has completed
//! is a key that was registered, since the domain completes a
//! generation's row in the transaction that registers the key and in no
//! other, so a request that is entered again on such a row is given
//! that key from the row; and a row whose files were retained for
//! inspection is final, and nothing continues it.
//!
//! The passphrase of a new key is given to the one domain call and to
//! nothing else. It is no field of the digest, and no record, journal
//! row, result or error holds it or anything made from it.

use std::{
    cell::Cell,
    path::{Path, PathBuf},
};

use crate::results::{ResultCode, Scope};

use super::{
    super::{
        super::{
            OperationFamily, ReadError, RegisterSharedKeyOutcome, RegisterSharedKeyRequest,
            RepositoryService, SharedKeyId, SharedKeyOwnership, SharedKeySelectionOutcome,
            keys::{
                GenerateSharedKeyOutcome, GenerateSharedKeyRequest, KeyMaterialPhase,
                KeyMaterialRecovery, KeyProtection, KeyStore,
            },
            recovery::{FinalKind, PendingOperation},
        },
        command::Command,
        dto::{KeyGenerationState, KeyMutationDto, MutationDataDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        outcome,
    },
    Accepted, Answer, Binding, CallContext, EndedWithoutCheckpoint, Entered, Place, Prepared, Ran,
    Standing,
};
use crate::repository::{SharedKeyRegistration, recovery::JournalRow};

/// The input of `key import`: a private key file the user keeps, to
/// register under `label`, and its public key file if it has one.
/// Nothing is copied and neither file is changed: the registration names
/// them where they are.
pub struct KeyImportInput {
    pub label: String,
    pub private_key_path: PathBuf,
    pub public_key_path: Option<PathBuf>,
}

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

/// A path as the digest reads it: the bytes the caller gave, whatever
/// their encoding. A path is fed to the digest and stored nowhere by the
/// boundary.
fn path_field(path: &Path) -> FieldValue<'_> {
    FieldValue::Bytes(path.as_os_str().as_encoded_bytes())
}

/// What a change to the key registry asks for.
enum Change {
    Import {
        label: String,
        private_key_path: PathBuf,
        public_key_path: Option<PathBuf>,
    },
    Select(SharedKeyId),
    Clear,
    Remove(SharedKeyId),
}

/// `key import`, `key select`, `key clear` and `key remove`: one binding,
/// since each is one registry transaction with no operation.
pub(crate) struct KeyRegistryBinding {
    change: Change,
}

impl From<KeyImportInput> for KeyRegistryBinding {
    fn from(input: KeyImportInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let KeyImportInput {
            label,
            private_key_path,
            public_key_path,
        } = input;
        Self {
            change: Change::Import {
                label,
                private_key_path,
                public_key_path,
            },
        }
    }
}

impl From<KeySelectInput> for KeyRegistryBinding {
    fn from(input: KeySelectInput) -> Self {
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
    /// The key the request names, if it names one. An import names a
    /// source, and learns its key from the registry.
    fn id(&self) -> Option<SharedKeyId> {
        match self.change {
            Change::Select(id) | Change::Remove(id) => Some(id),
            Change::Import { .. } | Change::Clear => None,
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
        match &self.change {
            Change::Import {
                label,
                private_key_path,
                public_key_path,
            } => {
                let registration = service.register_shared_key(RegisterSharedKeyRequest {
                    label: label.clone(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: private_key_path.clone(),
                    public_key_path: public_key_path.clone(),
                });
                match registration {
                    Ok(registration) => {
                        let code = outcome::register_key_outcome_code(&registration);
                        match registration {
                            RegisterSharedKeyOutcome::Registered(registration) => {
                                answer(code, registered(&registration))
                            }
                            // The key that holds the source, with its
                            // own label and not the request's. A read
                            // of it that fails is the request's answer,
                            // naming the key: nothing was registered,
                            // and a no-op is not stored for a key the
                            // registry could not show.
                            RegisterSharedKeyOutcome::SourceAlreadyRegistered { existing } => {
                                match service.show_key(existing) {
                                    Ok(key) => answer(
                                        code,
                                        key_data(
                                            Some(existing),
                                            Some(&key.label),
                                            key.fingerprint.as_deref(),
                                        ),
                                    ),
                                    Err(error) => Answer::of_read(&error).with_data(key_data(
                                        Some(existing),
                                        None,
                                        None,
                                    )),
                                }
                            }
                        }
                    }
                    Err(error) => failed(error.kind),
                }
            }
            // The domain selects a key that is already selected and says
            // the same of both, so the registry is read first. A key
            // that already is selected is left as it is.
            &Change::Select(id) => match service.show_key(id) {
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
            &Change::Remove(id) => match service.unregister_shared_key(id) {
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
            Change::Import { .. } => Command::KeyImport,
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

    /// The key, or nothing for `key clear`, which names none, and for
    /// `key import`: a record holds its target as text, and the source of
    /// an import is a path in the user's home. The source is a field of
    /// the digest instead.
    fn target(&self) -> String {
        self.id().map(|id| id.to_string()).unwrap_or_default()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        match &self.change {
            Change::Import {
                label,
                private_key_path,
                public_key_path,
            } => builder
                .field("label", FieldValue::Text(label))
                .field("private_key_path", path_field(private_key_path))
                .field(
                    "public_key_path",
                    match public_key_path {
                        Some(path) => path_field(path),
                        None => FieldValue::Absent,
                    },
                ),
            // The command and the target are what the digest is begun
            // with, and these hold nothing else.
            Change::Select(_) | Change::Clear | Change::Remove(_) => builder,
        }
    }

    /// A key command names no repository, item, branch or worktree.
    fn scope(&self, _repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope::default()
    }

    /// A key that is not registered is refused here, before the request
    /// is accepted. An import is validated by the domain call alone, which
    /// writes nothing for an input it refuses. `key remove` depends on it: the domain reports a key
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
            Change::Import { .. } | Change::Clear => {}
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

/// The input of `key generate`: the key store the pair is written into,
/// the label the key is registered under, and how its private key is
/// protected.
///
/// `protection` can hold the new key's passphrase, so this is neither
/// printed nor serialized:
///
/// ```compile_fail
/// # use manyhands::repository::KeyGenerateInput;
/// # fn assert_serialize<T: serde::Serialize>() {}
/// assert_serialize::<KeyGenerateInput>();
/// ```
/// ```compile_fail
/// # use manyhands::repository::KeyGenerateInput;
/// # fn assert_debug<T: std::fmt::Debug>() {}
/// assert_debug::<KeyGenerateInput>();
/// ```
pub struct KeyGenerateInput {
    pub store: KeyStore,
    pub label: String,
    pub protection: KeyProtection,
}

/// Whether the row of a generation says that a key file was written and
/// the generation has not ended: a change that no effect shows, with work
/// remaining.
pub(crate) fn written_and_unfinished(journal: Option<&JournalRow>) -> bool {
    match journal {
        Some(JournalRow::Pending(PendingOperation::KeyMaterial { phase })) => match phase {
            KeyMaterialPhase::PrivateWritten | KeyMaterialPhase::PairWritten => true,
            // Nothing is known to be written, or the row is a deletion's.
            KeyMaterialPhase::Reserved
            | KeyMaterialPhase::Prepared
            | KeyMaterialPhase::PrivateRemoved
            | KeyMaterialPhase::FilesRemoved
            | KeyMaterialPhase::Completed
            | KeyMaterialPhase::RetainedForInspection => false,
        },
        Some(
            JournalRow::Absent
            | JournalRow::Pending(
                PendingOperation::Local { state: _, step: _ }
                | PendingOperation::Remote { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            },
        )
        | None => false,
    }
}

/// Where a generation stands, as a result names it. `None` for a phase
/// only a deletion passes through.
fn generation_state(phase: KeyMaterialPhase) -> Option<KeyGenerationState> {
    match phase {
        KeyMaterialPhase::Reserved => Some(KeyGenerationState::Reserved),
        KeyMaterialPhase::PrivateWritten => Some(KeyGenerationState::PrivateWritten),
        KeyMaterialPhase::PairWritten => Some(KeyGenerationState::PairWritten),
        KeyMaterialPhase::Completed => Some(KeyGenerationState::Completed),
        KeyMaterialPhase::RetainedForInspection => Some(KeyGenerationState::RetainedForInspection),
        KeyMaterialPhase::Prepared
        | KeyMaterialPhase::PrivateRemoved
        | KeyMaterialPhase::FilesRemoved => None,
    }
}

pub(crate) struct KeyGenerateBinding {
    store: KeyStore,
    label: String,
    /// Whether the private key is to be protected by a passphrase: what
    /// the digest holds of the protection.
    protected: bool,
    /// The protection itself, which can hold the passphrase. It is taken
    /// by the one domain call a binding makes, and is read nowhere else.
    protection: Cell<Option<KeyProtection>>,
}

impl From<KeyGenerateInput> for KeyGenerateBinding {
    fn from(input: KeyGenerateInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let KeyGenerateInput {
            store,
            label,
            protection,
        } = input;
        Self {
            store,
            label,
            protected: match &protection {
                KeyProtection::Unencrypted => false,
                KeyProtection::Passphrase(_) => true,
            },
            protection: Cell::new(Some(protection)),
        }
    }
}

impl KeyGenerateBinding {
    /// What the request reports of its key: the ID the generation
    /// reserved, once it has one, and the label the request gives.
    fn data(
        &self,
        id: Option<SharedKeyId>,
        fingerprint: Option<&str>,
        generation: Option<KeyGenerationState>,
    ) -> MutationDataDto {
        MutationDataDto::Key(KeyMutationDto {
            id: id.map(|id| id.to_string()),
            label: Some(self.label.clone()),
            fingerprint: fingerprint.map(str::to_owned),
            generation,
        })
    }

    /// A generation that stopped short of a registered key, as the
    /// domain reports it. Its key ID is reserved and its files may be
    /// written, which no effect shows: the result is `partial`.
    ///
    /// The record is finished only when the domain has made the result
    /// final, by retaining the files for inspection. In every other
    /// phase the same request continues the generation, and the record
    /// stays accepted for it. That includes a call that registered its
    /// key and then failed: the row has completed, a record released for
    /// it would have the next call generate a second key for the same
    /// request, and the same request is given the key from the row.
    fn interrupted(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        recovery: &KeyMaterialRecovery,
        code: ResultCode,
    ) -> Ran {
        let KeyMaterialRecovery {
            operation_id: _,
            key_id,
            action: _,
            phase,
            // Read through `operation show`, with the operation's state.
            failure_code: _,
            recovery_action: _,
        } = recovery;
        let answer = Answer {
            local_change: true,
            ..Answer::stopped(code).with_data(self.data(
                Some(*key_id),
                None,
                generation_state(*phase),
            ))
        };
        let standing = match phase {
            // Final by the row's own word, read after the call. A row
            // that does not say so, or cannot be read, finishes nothing.
            KeyMaterialPhase::RetainedForInspection => Standing::Stopped {
                journal: accepted.journal_row(service).filter(|journal| {
                    matches!(
                        journal,
                        JournalRow::Final {
                            kind: FinalKind::RetainedForInspection,
                            owes_work: _,
                            checkpointed: _,
                        }
                    )
                }),
            },
            KeyMaterialPhase::Reserved
            | KeyMaterialPhase::PrivateWritten
            | KeyMaterialPhase::PairWritten
            | KeyMaterialPhase::Completed
            | KeyMaterialPhase::Prepared
            | KeyMaterialPhase::PrivateRemoved
            | KeyMaterialPhase::FilesRemoved => Standing::Owed,
        };
        Ran { answer, standing }
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to. It is the same call for a first attempt and for a retry:
    /// the domain takes up the operation where its row stands.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        // A binding makes one domain call. Without the protection there
        // is none to make, and nothing is settled on that.
        let Some(protection) = self.protection.take() else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError)
                    .with_data(self.data(None, None, None)),
                standing: Standing::Stopped { journal: None },
            };
        };
        let result = service.generate_shared_key(
            &self.store,
            GenerateSharedKeyRequest {
                operation_id: accepted.operation_id,
                label: self.label.clone(),
                protection,
            },
        );
        match result {
            Ok(generated) => {
                let code = outcome::generate_key_outcome_code(&generated);
                let created = |code, registration: &SharedKeyRegistration| Ran {
                    answer: Answer::stopped(code).with_data(self.data(
                        Some(registration.id),
                        registration.public_key_fingerprint.as_deref(),
                        None,
                    )),
                    standing: Standing::Final,
                };
                match generated {
                    GenerateSharedKeyOutcome::Created(registration) => created(code, &registration),
                    // The operation is the request's own, so a key it
                    // had already created is the key this request
                    // generated, and no no-op: the work of a call of the
                    // same request that ran beside this one.
                    GenerateSharedKeyOutcome::AlreadyCreated(registration) => {
                        created(ResultCode::Ok, &registration)
                    }
                    GenerateSharedKeyOutcome::RecoveryRequired(recovery) => {
                        self.interrupted(service, accepted, &recovery, code)
                    }
                }
            }
            Err(error) => {
                // Only the kind is read.
                let code = outcome::key_material_error_code(error.kind);
                let journal = accepted.journal_row(service);
                let answer = Answer {
                    local_change: written_and_unfinished(journal.as_ref()),
                    ..Answer::stopped(code).with_data(self.data(None, None, None))
                };
                let standing = match journal {
                    // No row, on a call that entered a record another
                    // call accepted. That call may be running still,
                    // between its acceptance and its reservation, and
                    // this one cannot know that it has stopped. Were the
                    // record released, that call's key would belong to
                    // no record, and this request sent again would be a
                    // new one and generate a second key. The record is
                    // kept: the same request, under the same operation
                    // ID, finds the key or generates the one.
                    Some(JournalRow::Absent) if accepted.attempt > 1 => Standing::Unconfirmed,
                    // The generation never began, on the call that
                    // accepted it, or has not ended: the row says whether
                    // anything is in flight. A first call's release is
                    // conditional on its attempt, and changes nothing
                    // once another call has entered.
                    Some(JournalRow::Absent | JournalRow::Pending(_)) | None => {
                        Standing::Stopped { journal }
                    }
                    // The generation has ended, and this call failed all
                    // the same: the store was busy over files that were
                    // retained, or a call beside this one completed the
                    // row. The error is no result of the generation.
                    // Nothing is stored and nothing is deleted; the same
                    // request is answered from the row.
                    Some(JournalRow::Final {
                        kind: _,
                        owes_work: _,
                        checkpointed: _,
                    }) => Standing::Unconfirmed,
                };
                Ran { answer, standing }
            }
        }
    }
}

impl Binding for KeyGenerateBinding {
    fn command(&self) -> Command {
        Command::KeyGenerate
    }

    /// The key journal records phases and no checkpoint step.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Application
    }

    /// A generation names no key: its key's ID is the domain's to
    /// reserve.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // Where the pair is written. The domain refuses an operation
            // ID that is taken up again with another store.
            store: _,
            label,
            protected,
            // Never a field: two requests that differ only in their
            // passphrase are the same request.
            protection: _,
        } = self;
        builder
            .field("label", FieldValue::Text(label))
            .field("protected", FieldValue::Flag(*protected))
    }

    fn scope(&self, _repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope::default()
    }

    /// A label the domain refuses is refused here, before the request is
    /// accepted and given an operation.
    fn prepare(&mut self, _call: &CallContext<'_>) -> Result<Prepared, Answer> {
        if self.label.trim().is_empty() {
            return Err(
                Answer::stopped(ResultCode::InvalidInput).with_data(self.data(None, None, None))
            );
        }
        Ok(Prepared {
            family: Some(OperationFamily::KeyMaterial),
            position: no_position(),
            expected: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// A row that has completed is a key that was registered: the domain
    /// completes a generation's row in the transaction that registers its
    /// key and in no other. The request is finished here with that key,
    /// read from the row, and no domain call is made: the call would
    /// check the key's files and its registration as they are now, and a
    /// key that was since removed, or whose files were deleted, would
    /// have it refuse for good a request that succeeded.
    ///
    /// For every other row the domain operation is called, and its answer
    /// is the evidence: with no row it generates, on a pair that was
    /// written it verifies and registers, on anything less it retains the
    /// files and makes that final, and on a row already retained it says
    /// so again.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match journal {
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: _,
                checkpointed: _,
            } => {
                // What cannot be read answers the call and leaves the
                // record as it is.
                let unread = |answer: Answer| Ran {
                    answer: answer.with_data(self.data(None, None, None)),
                    standing: Standing::NotRun {
                        journal: journal.clone(),
                    },
                };
                match call.service.completed_generation(accepted.operation_id) {
                    Ok(Some((id, fingerprint))) => Err(Ran {
                        answer: Answer::stopped(ResultCode::Ok).with_data(self.data(
                            Some(id),
                            fingerprint.as_deref(),
                            None,
                        )),
                        standing: Standing::Final,
                    }),
                    // The row is no completed generation after all.
                    Ok(None) => Err(unread(Answer::stopped(ResultCode::InternalError))),
                    Err(error) => Err(unread(Answer::of_read(&error))),
                }
            }
            JournalRow::Absent
            | JournalRow::Pending(PendingOperation::KeyMaterial { phase: _ })
            | JournalRow::Final {
                kind: FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => Ok(Entered::Unfinished),
            // No row of the key journal reads so. Nothing is run or
            // settled on one that does.
            JournalRow::Pending(
                PendingOperation::Local { state: _, step: _ }
                | PendingOperation::Remote { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Cancelled,
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
