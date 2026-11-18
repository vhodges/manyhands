//! The shared-key commands. They belong to the application and to no
//! repository: a request of one is recorded under the scope `application`,
//! and its results name no repository, item, branch or worktree.
//!
//! A key is no canonical content and no commit, so no effect shows one:
//! every effect is `not_requested`, `data` names the key, and the outcome
//! says whether anything changed.
//!
//! `key import`, `key select`, `key clear` and `key remove` change the
//! key registry and nothing else. The domain call of each is one
//! transaction: it happened or it did not, and nothing is ever in flight
//! under it.
//!
//! An import, a selection and a clearing can each be made a second time:
//! a source can be registered again once its key is removed, and a
//! selection can be put back. So each is an operation, of the key
//! journal's family, and the transaction that makes the change records
//! it against the operation's ID. A request that is entered again is
//! answered from that record, with the key its first call named,
//! whatever has been done to the key or to the selection since, and
//! changes nothing. One whose operation recorded nothing makes its change
//! in a transaction that first compares the setting with what the
//! request observed when it was accepted: a setting someone has changed
//! since is an `external_change`, and is not overwritten.
//!
//! `key remove` has no operation and records nothing. It cannot be made
//! twice: a key's ID is never registered again, so a retry that finds
//! the key gone has nothing to do and nothing to undo.
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
//! `key delete` deletes the two files of a key Manyhands generated and
//! removes its registration, as one operation in the same journal. It is
//! the one key command that needs confirmation; see `KeyDeleteBinding`.
//!
//! The passphrase of a new key is given to the one domain call and to
//! nothing else. It is no field of the digest, and no record, journal
//! row, result or error holds it or anything made from it.

use crate::repository::mutation::confirm::{self, Row, Seen};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
};

use crate::results::{ResultCode, Scope};

use super::{
    super::{
        super::{
            ExpectedPathObservation, GeneratedKeyDeletionPreflight, OperationFamily,
            PrivateKeySourceState, PublicKeyMetadataState, ReadError, RegisterSharedKeyRequest,
            RepositoryError, RepositoryErrorKind, RepositoryService, SharedKeyId,
            SharedKeyOwnership,
            keys::{
                self, DeleteGeneratedKeyOutcome, GenerateSharedKeyOutcome,
                GenerateSharedKeyRequest, GeneratedKeyDeletionReview, KeyMaterialErrorKind,
                KeyMaterialPhase, KeyMaterialRecovery, KeyProtection, KeyRegistryChange,
                KeyRegistryMark, KeyRegistrySetting, KeyStore, MarkedRegistryChange,
            },
            recovery::{FinalKind, PendingOperation},
        },
        command::Command,
        dto::{
            KeyDeletionPreviewDto, KeyDeletionState, KeyGenerationState, KeyMutationDto,
            MutationDataDto, PreviewDetailsDto, PreviewEffect,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{Observation, ObservationDigest},
        outcome,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused,
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
        deletion: None,
    })
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
/// since each is one registry transaction.
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

    /// Whether the command's change is recorded against an operation;
    /// see the module's text. Only a removal's is not.
    fn recorded(&self) -> bool {
        match self.change {
            Change::Import { .. } | Change::Select(_) | Change::Clear => true,
            Change::Remove(_) => false,
        }
    }

    /// The answer a recorded change gives: the key as the record names
    /// it, and whether anything changed. It is the same answer for the
    /// call that made the change and for every later call of the request.
    fn answered(change: &KeyRegistryChange) -> Answer {
        let KeyRegistryChange {
            // The request's own, as the domain checked.
            action: _,
            key_id,
            label,
            public_key_fingerprint,
            // Read by the code.
            changed: _,
        } = change;
        Answer::stopped(outcome::key_registry_change_code(change)).with_data(key_data(
            *key_id,
            label.as_deref(),
            public_key_fingerprint.as_deref(),
        ))
    }

    /// The one domain call and what it came to, settled.
    ///
    /// A change that is recorded is made under the request's operation:
    /// one transaction answers from a record already there, compares the
    /// setting with `expected` when that is given, makes the change and
    /// records it. What is recorded is the request's end state, and so is
    /// a setting that has moved: the request is finished as an external
    /// change, since made later its change would put an older choice
    /// over a later one. Anything else stopped the request with nothing
    /// done.
    ///
    /// `expected` is given by a call that enters the request again, and
    /// not by its first call, which observed the setting a moment before.
    fn call(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        expected: Option<ExpectedPathObservation>,
    ) -> Ran {
        let mark = KeyRegistryMark {
            operation_id: accepted.operation_id,
            expected,
        };
        let made = match &self.change {
            Change::Import {
                label,
                private_key_path,
                public_key_path,
            } => service.register_shared_key_under(
                &mark,
                RegisterSharedKeyRequest {
                    label: label.clone(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: private_key_path.clone(),
                    public_key_path: public_key_path.clone(),
                },
            ),
            &Change::Select(id) => service.select_shared_key_under(&mark, id),
            Change::Clear => service.clear_shared_key_selection_under(&mark),
            &Change::Remove(id) => return self.remove(service, accepted, id),
        };
        self.settled(service, accepted, made)
    }

    /// What a change made under the request's operation came to.
    fn settled(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        made: Result<MarkedRegistryChange, RepositoryError>,
    ) -> Ran {
        let code = match &made {
            Ok(made) => outcome::marked_registry_change_code(made),
            // Only the kind of a domain error is read.
            Err(error) => outcome::repository_error_code(error.kind),
        };
        match made {
            Ok(MarkedRegistryChange::Recorded(change)) => Ran {
                answer: Self::answered(&change),
                standing: Standing::Final,
            },
            Ok(MarkedRegistryChange::Moved) => Ran {
                answer: Answer::stopped(code).with_data(self.named()),
                standing: Standing::Final,
            },
            Ok(MarkedRegistryChange::NotRegistered) | Err(_) => {
                let journal = accepted.journal_row(service);
                let standing = match journal {
                    // Nothing is recorded, on a call that entered a
                    // record another call accepted. The record is what
                    // holds the setting the request observed and the
                    // operation its change is recorded against. Were it
                    // released, the request sent again would be a new
                    // one: it would observe the setting as it is by then
                    // and put its choice over a later one, and a first
                    // call still on its way would record its change
                    // under an operation no record names. The record is
                    // kept, whatever the failure: the same request is
                    // its way out.
                    Some(JournalRow::Absent) if accepted.attempt > 1 => Standing::Unconfirmed,
                    // Nothing is recorded, on the call that accepted the
                    // request: nothing was done, and nothing is in
                    // flight. Its release is conditional on its attempt,
                    // and changes nothing once another call has entered.
                    Some(JournalRow::Absent | JournalRow::Pending(_)) | None => {
                        Standing::Stopped { journal }
                    }
                    // The change is recorded, by a call of the request
                    // that ran beside this one, and this call failed all
                    // the same. The failure is no result of the change:
                    // nothing is stored and nothing is deleted, and the
                    // same request is answered from the record.
                    Some(JournalRow::Final {
                        kind: _,
                        owes_work: _,
                        checkpointed: _,
                    }) => Standing::Unconfirmed,
                };
                Ran {
                    answer: Answer::stopped(code).with_data(self.named()),
                    standing,
                }
            }
        }
    }

    /// `key remove`: the one domain call, which is its own evidence of
    /// what is done. A key that is gone is the request's end state.
    fn remove(&self, service: &RepositoryService, accepted: &Accepted, id: SharedKeyId) -> Ran {
        let code = match service.unregister_shared_key(id) {
            Ok(removal) => outcome::unregister_key_outcome_code(removal),
            // Only the kind of a domain error is read.
            Err(error) => outcome::repository_error_code(error.kind),
        };
        Ran {
            answer: Answer::stopped(code).with_data(self.named()),
            standing: match code.failure_class() {
                None => Standing::Final,
                Some(_) => Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            },
        }
    }

    /// The setting the request would change, as the registry holds it
    /// now: the selection for `key select` and `key clear`, and the
    /// registration of the source for `key import`. `None` for a command
    /// that is guarded without one: `key remove` is never made twice,
    /// since an ID is not registered again. `None` too for an import
    /// whose source the registry does not take as a path: the domain
    /// call refuses it, and nothing is observed of it.
    ///
    /// What is digested is a key ID or the absence of one, under the
    /// name of the setting; see `keys::observed_setting`. The same digest
    /// is made again by the domain, in the transaction of a retry's
    /// change.
    fn observe(
        &self,
        service: &RepositoryService,
    ) -> Result<Option<ExpectedPathObservation>, Answer> {
        let selected = || -> Result<Option<String>, Answer> {
            Ok(service
                .list_keys()
                .map_err(|error| self.unread(&error))?
                .items
                .into_iter()
                .find(|key| key.selected)
                .map(|key| key.id))
        };
        Ok(match &self.change {
            Change::Select(_) | Change::Clear => Some(keys::observed_setting(
                KeyRegistrySetting::Selection,
                selected()?.as_deref(),
            )),
            Change::Import {
                label: _,
                private_key_path,
                public_key_path: _,
            } => match service.registered_key_source(private_key_path) {
                Ok(registered) => Some(keys::observed_setting(
                    KeyRegistrySetting::Source,
                    registered.map(|id| id.to_string()).as_deref(),
                )),
                Err(error) if error.kind == RepositoryErrorKind::InvalidSharedKeySourcePath => None,
                Err(error) => {
                    return Err(Answer::stopped(outcome::repository_error_code(error.kind))
                        .with_data(self.named()));
                }
            },
            Change::Remove(_) => None,
        })
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

    /// A recorded registry change reads as a row that has completed, and
    /// is no authoring write: the row is the change, made. The record is
    /// kept and `reenter` answers from what was recorded.
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
        // The setting as this call finds it, for a retry whose operation
        // recorded nothing to compare: without this, a late retry would
        // make its change over one made since.
        let expected = self.observe(call.service)?;
        Ok(Prepared {
            observation: None,
            // The key journal's family: a recorded change is an
            // operation of the application's, as a generation is.
            family: self.recorded().then_some(OperationFamily::KeyMaterial),
            position: no_position(),
            expected,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted, None)
    }

    /// A change that is recorded against the request's operation was
    /// made: the request is finished here with what the record holds,
    /// and no domain call is made. The call would read the registry as it
    /// is now, and a key that was since removed, or a selection that was
    /// since changed, or changed and put back, would have it register
    /// the source again or put the request's choice over a later one.
    ///
    /// With nothing recorded no call of the request made its change. The
    /// domain call is made, and compares the setting with what the first
    /// call observed in the transaction that would change it; see `call`.
    ///
    /// A removal has no operation, so its row is absent whatever an
    /// earlier call did, and its domain call is made again.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match journal {
            JournalRow::Absent => Ok(Entered::Unfinished),
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: _,
                checkpointed: _,
            } if self.recorded() => {
                // What cannot be read answers the call and leaves the
                // record as it is.
                let unread = |answer: Answer| Ran {
                    answer: answer.with_data(self.named()),
                    standing: Standing::NotRun {
                        journal: journal.clone(),
                    },
                };
                match call.service.key_registry_change(accepted.operation_id) {
                    Ok(Some(change)) => Err(Ran {
                        answer: Self::answered(&change),
                        standing: Standing::Final,
                    }),
                    // The row is no recorded change after all.
                    Ok(None) => Err(unread(Answer::stopped(ResultCode::InternalError))),
                    Err(error) => Err(unread(Answer::of_read(&error))),
                }
            }
            // No row of a registry change reads so, and a removal has no
            // row. Nothing is run or settled on one that does.
            JournalRow::Pending(_)
            | JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            }),
        }
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    /// The setting is compared with what the request's first call
    /// observed, in the transaction of the change. A record that holds
    /// no observation, as one an earlier build accepted, makes its change
    /// as before.
    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted, accepted.expected.clone())
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
            deletion: None,
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
            observation: None,
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

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

/// The input of `key delete`: a key Manyhands generated, and the key
/// store that holds its files. The key's two files are deleted and its
/// registration removed.
#[derive(Clone)]
pub struct KeyDeleteInput {
    pub store: KeyStore,
    pub id: SharedKeyId,
}

/// Whether the row of a deletion says that a key file was removed and
/// the deletion has not ended: a change that no effect shows, with work
/// remaining.
pub(crate) fn removed_and_unfinished(journal: Option<&JournalRow>) -> bool {
    match journal {
        Some(JournalRow::Pending(PendingOperation::KeyMaterial { phase })) => match phase {
            KeyMaterialPhase::PrivateRemoved | KeyMaterialPhase::FilesRemoved => true,
            // No removal is recorded, or the row is a generation's.
            KeyMaterialPhase::Prepared
            | KeyMaterialPhase::Reserved
            | KeyMaterialPhase::PrivateWritten
            | KeyMaterialPhase::PairWritten
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

/// Where a deletion stands, as a result names it. `None` for a phase
/// only a generation passes through, and for one that has ended.
fn deletion_state(phase: KeyMaterialPhase) -> Option<KeyDeletionState> {
    match phase {
        KeyMaterialPhase::Prepared => Some(KeyDeletionState::Prepared),
        KeyMaterialPhase::PrivateRemoved => Some(KeyDeletionState::PrivateRemoved),
        KeyMaterialPhase::FilesRemoved => Some(KeyDeletionState::FilesRemoved),
        KeyMaterialPhase::Reserved
        | KeyMaterialPhase::PrivateWritten
        | KeyMaterialPhase::PairWritten
        | KeyMaterialPhase::Completed
        | KeyMaterialPhase::RetainedForInspection => None,
    }
}

/// Whether anything is at `path`: a file, a link or a directory. Only
/// "nothing is there" reads as absent; what cannot be read is taken to
/// be there, so that it is named and never deleted unnamed.
fn is_there(path: &Path) -> bool {
    !matches!(
        std::fs::symlink_metadata(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
}

/// The digest of a key's registration as the preflight of a deletion
/// reports it: what a confirmation of `key delete` stores, and what a
/// later observation is compared with. The command reads no identity.
///
/// With the registration goes whether each file it names is there:
/// that is read from the file system without opening the key store or
/// any file, and it is what the preview's list of deletions is made
/// from.
///
/// A key that cannot be deleted is observed as what stands in the way,
/// so that it never equals the observation of one that can.
fn registration_digest(preflight: &GeneratedKeyDeletionPreflight) -> ObservationDigest {
    let command = Command::KeyDelete.name();
    let observation = Observation::of(command);
    let observation = match preflight {
        GeneratedKeyDeletionPreflight::ConfirmationRequired(registration) => {
            let SharedKeyRegistration {
                id,
                label,
                ownership,
                private_key_path,
                public_key_path,
                public_key_fingerprint,
                private_source_state,
                public_metadata_state,
                selected,
            } = registration;
            let optional = |observation: Observation, name: &str, value: Option<&str>| match value {
                Some(value) => observation.text(name, value),
                None => observation.absent(name),
            };
            let observation = observation
                .text("key", "deletable")
                .text("id", &id.to_string())
                .text("label", label)
                .text(
                    "ownership",
                    match ownership {
                        SharedKeyOwnership::Imported => "imported",
                        SharedKeyOwnership::Generated => "generated",
                    },
                )
                .text("private_key_path", &private_key_path.to_string_lossy());
            let observation = optional(
                observation,
                "public_key_path",
                public_key_path
                    .as_deref()
                    .map(Path::to_string_lossy)
                    .as_deref(),
            );
            optional(
                observation,
                "fingerprint",
                public_key_fingerprint.as_deref(),
            )
            .text(
                "private_source_state",
                match private_source_state {
                    PrivateKeySourceState::Available => "available",
                    PrivateKeySourceState::Missing => "missing",
                    PrivateKeySourceState::Unavailable => "unavailable",
                },
            )
            .text(
                "public_metadata_state",
                match public_metadata_state {
                    PublicKeyMetadataState::NotProvided => "not_provided",
                    PublicKeyMetadataState::FingerprintAvailable => "available",
                    PublicKeyMetadataState::Unavailable => "unavailable",
                },
            )
            .text("selected", if *selected { "yes" } else { "no" })
            .text(
                "private_key_file",
                if is_there(private_key_path) {
                    "present"
                } else {
                    "absent"
                },
            )
            .text(
                "public_key_file",
                if public_key_path.as_deref().is_some_and(is_there) {
                    "present"
                } else {
                    "absent"
                },
            )
        }
        GeneratedKeyDeletionPreflight::NotRegistered => observation.text("key", "not_registered"),
        GeneratedKeyDeletionPreflight::ImportedKey => observation.text("key", "imported"),
        GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared => {
            observation.text("key", "selected")
        }
    };
    ObservationDigest::of_state(command, observation.finish())
}

/// `key delete`: the two files of a key Manyhands generated are deleted
/// from the user's key store, and the key is taken out of the key list.
/// It needs confirmation, and belongs to the application as every key
/// command does. Its operation is in the key journal.
///
/// **What its preview covers.** The key's registration, as the preflight
/// of a deletion reports it, and whether each of its two files is there.
/// The key's files are not opened or read for a preview:
/// the domain's own review reads them when the request runs, and a file
/// that is not the one Manyhands wrote is its `external_change`.
///
/// **The review.** The domain deletes only against a review of the key
/// it made itself, which cannot be stored and is used once. The binding
/// obtains one before each domain call that needs one: in `prepare`, and
/// on a retry in `reenter`. What the request's consent was to is
/// compared with the registration that review holds.
pub(crate) struct KeyDeleteBinding {
    store: KeyStore,
    id: SharedKeyId,
    /// The key's registration, once the request has read it: what its
    /// results name the key by, and where its files are.
    registration: Option<SharedKeyRegistration>,
    /// The review the next domain call is given. It is taken by that
    /// call: a review is used once.
    review: Cell<Option<GeneratedKeyDeletionReview>>,
}

impl From<KeyDeleteInput> for KeyDeleteBinding {
    fn from(input: KeyDeleteInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let KeyDeleteInput { store, id } = input;
        Self {
            store,
            id,
            registration: None,
            review: Cell::new(None),
        }
    }
}

impl KeyDeleteBinding {
    /// What the request reports of its key: the key it names, with the
    /// label and the fingerprint of its registration when the request
    /// read it, and where its deletion stands when it has not ended.
    fn data(&self, deletion: Option<KeyDeletionState>) -> MutationDataDto {
        let registration = self.registration.as_ref();
        MutationDataDto::Key(KeyMutationDto {
            id: Some(self.id.to_string()),
            label: registration.map(|registration| registration.label.clone()),
            fingerprint: registration
                .and_then(|registration| registration.public_key_fingerprint.clone()),
            generation: None,
            deletion,
        })
    }

    fn answer(&self, code: ResultCode) -> Answer {
        Answer::stopped(code).with_data(self.data(None))
    }

    /// The answer to a failure of the review or of the deletion.
    ///
    /// A key file that is no longer the one Manyhands wrote is refused
    /// for as long as the file is what it is. A preview observes the
    /// registration and not the files, so a new one shows the same
    /// deletion and its request is refused the same way: the preview
    /// the code offers a confirmed command is not offered, and nothing
    /// is.
    fn refusal(&self, kind: KeyMaterialErrorKind) -> Answer {
        Answer {
            offers_nothing: kind == KeyMaterialErrorKind::SourceChanged,
            ..self.answer(Self::failure_code(kind))
        }
    }

    /// The code of a failure of the review or of the deletion. A
    /// deletion that says it lacks its review was called without one by
    /// the binding, which is the binding's fault and no state a caller
    /// can act on.
    fn failure_code(kind: KeyMaterialErrorKind) -> ResultCode {
        match kind {
            KeyMaterialErrorKind::ConfirmationRequired => ResultCode::InternalError,
            KeyMaterialErrorKind::InvalidLabel
            | KeyMaterialErrorKind::InvalidPassphrase
            | KeyMaterialErrorKind::RegistryUnavailable
            | KeyMaterialErrorKind::Busy
            | KeyMaterialErrorKind::NotRegistered
            | KeyMaterialErrorKind::SelectedKeyMustBeCleared
            | KeyMaterialErrorKind::ImportedKey
            | KeyMaterialErrorKind::OwnershipUnverified
            | KeyMaterialErrorKind::SourceMissing
            | KeyMaterialErrorKind::SourceUnreadable
            | KeyMaterialErrorKind::NotRegularFile
            | KeyMaterialErrorKind::UnsafePath
            | KeyMaterialErrorKind::InvalidGeneratedKey
            | KeyMaterialErrorKind::UnlockFailed
            | KeyMaterialErrorKind::SourceChanged
            | KeyMaterialErrorKind::SelectionChanged
            | KeyMaterialErrorKind::OperationMismatch
            | KeyMaterialErrorKind::HomeUnavailable
            | KeyMaterialErrorKind::ProtectionUnavailable
            | KeyMaterialErrorKind::RandomnessUnavailable
            | KeyMaterialErrorKind::GenerationFailed
            | KeyMaterialErrorKind::StorageUnavailable => outcome::key_material_error_code(kind),
        }
    }

    /// The preflight of the deletion: the key's registration, or what
    /// stands in the way of deleting it. It reads the key registry and
    /// nothing else. A key it finds deletable is remembered.
    fn preflight(
        &mut self,
        service: &RepositoryService,
    ) -> Result<GeneratedKeyDeletionPreflight, Answer> {
        let preflight = service
            .preflight_generated_key_deletion(self.id)
            .map_err(|error| refused(error).with_data(self.data(None)))?;
        match &preflight {
            GeneratedKeyDeletionPreflight::ConfirmationRequired(registration) => {
                self.registration = Some(registration.clone());
            }
            GeneratedKeyDeletionPreflight::NotRegistered
            | GeneratedKeyDeletionPreflight::ImportedKey
            | GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared => {}
        }
        Ok(preflight)
    }

    /// A review of the key from the domain, kept for the next domain
    /// call, and the digest of the registration it holds. An `Err` is the
    /// domain's refusal to review the key: for a key file that is no
    /// longer the one Manyhands wrote, `external_change`.
    fn review(&mut self, service: &RepositoryService) -> Result<ObservationDigest, Answer> {
        match service.review_generated_key_deletion(&self.store, self.id) {
            Ok(review) => {
                let registration = review.registration().clone();
                self.review.set(Some(review));
                self.registration = Some(registration.clone());
                Ok(registration_digest(
                    &GeneratedKeyDeletionPreflight::ConfirmationRequired(registration),
                ))
            }
            // Only the kind is read.
            Err(error) => Err(self.refusal(error.kind)),
        }
    }

    /// What the request observes before a domain call that deletes: the
    /// digest of the key's registration, from the domain's review when
    /// the key can be deleted, which is then kept for the call, and from
    /// the preflight when it cannot.
    fn observe(&mut self, service: &RepositoryService) -> Result<ObservationDigest, Answer> {
        match self.preflight(service)? {
            GeneratedKeyDeletionPreflight::ConfirmationRequired(_) => self.review(service),
            preflight @ (GeneratedKeyDeletionPreflight::NotRegistered
            | GeneratedKeyDeletionPreflight::ImportedKey
            | GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared) => {
                Ok(registration_digest(&preflight))
            }
        }
    }

    /// Whether a file the key's registration names is not there. Under a
    /// deletion that has begun and not ended, that is a file it removed,
    /// also when the phase that says so was never recorded.
    fn a_file_is_gone(&self) -> bool {
        self.registration.as_ref().is_some_and(|registration| {
            !is_there(&registration.private_key_path)
                || registration
                    .public_key_path
                    .as_deref()
                    .is_some_and(|path| !is_there(path))
        })
    }

    /// Whether the request's deletion removed a file and has not ended:
    /// its row is pending, and holds a phase that says so or a file of
    /// the key is gone.
    fn removed(&self, journal: Option<&JournalRow>) -> bool {
        removed_and_unfinished(journal)
            || (matches!(
                journal,
                Some(JournalRow::Pending(PendingOperation::KeyMaterial {
                    phase: _
                }))
            ) && self.a_file_is_gone())
    }

    /// The one domain call, under the recorded operation ID, with the
    /// review that was obtained for it, if one was, and what it came to.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let result = service.delete_generated_key(
            &self.store,
            accepted.operation_id,
            self.review.take(),
            // The request's confirmation was accepted with its record.
            true,
        );
        match result {
            Ok(DeleteGeneratedKeyOutcome::RecoveryRequired(recovery)) => {
                let KeyMaterialRecovery {
                    operation_id: _,
                    key_id: _,
                    action: _,
                    phase,
                    // Read through `operation show`, with the
                    // operation's state.
                    failure_code: _,
                    recovery_action: _,
                } = recovery;
                // The deletion began and did not end: the same request
                // completes it, under the same operation, and the record
                // stays accepted for that.
                let removed = match phase {
                    KeyMaterialPhase::PrivateRemoved | KeyMaterialPhase::FilesRemoved => true,
                    KeyMaterialPhase::Prepared
                    | KeyMaterialPhase::Reserved
                    | KeyMaterialPhase::PrivateWritten
                    | KeyMaterialPhase::PairWritten
                    | KeyMaterialPhase::Completed
                    | KeyMaterialPhase::RetainedForInspection => self.a_file_is_gone(),
                };
                Ran {
                    answer: Answer {
                        local_change: removed,
                        ..Answer::stopped(ResultCode::RecoveryRequired)
                            .with_data(self.data(deletion_state(phase)))
                    },
                    standing: Standing::Owed,
                }
            }
            Ok(
                outcome @ (DeleteGeneratedKeyOutcome::Deleted
                | DeleteGeneratedKeyOutcome::AlreadyDeleted),
            ) => Ran {
                answer: self.answer(outcome::delete_key_outcome_code(&outcome)),
                standing: Standing::Final,
            },
            // Not what a confirmed deletion returns. Nothing is settled
            // on it but by what the journal says.
            Ok(DeleteGeneratedKeyOutcome::Cancelled) => Ran {
                answer: self.answer(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            },
            Err(error) => {
                // Only the kind is read.
                let journal = accepted.journal_row(service);
                let answer = Answer {
                    local_change: self.removed(journal.as_ref()),
                    ..self.refusal(error.kind)
                };
                let standing = match journal {
                    // The deletion never began, or has not ended: the
                    // row says whether anything is in flight.
                    Some(JournalRow::Absent | JournalRow::Pending(_)) | None => {
                        Standing::Stopped { journal }
                    }
                    // The deletion has ended, and this call failed all
                    // the same. The error is no result of the deletion:
                    // nothing is stored and nothing is deleted, and the
                    // same request is answered from the row.
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

impl Binding for KeyDeleteBinding {
    fn command(&self) -> Command {
        Command::KeyDelete
    }

    /// The key journal records phases and no checkpoint step.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Application
    }

    fn target(&self) -> String {
        self.id.to_string()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // Where the key's files are. The domain refuses an operation
            // ID that is taken up again with another store.
            store: _,
            // The target, which the digest is begun with.
            id: _,
            // Read by the request, and no part of it.
            registration: _,
            review: _,
        } = self;
        builder
    }

    /// A key command names no repository, item, branch or worktree.
    fn scope(&self, _repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope::default()
    }

    /// Observes: the registration is read, and for a key that can be
    /// deleted the domain's review is obtained for the call. No check is
    /// made of what the preview covers: the boundary compares the digest
    /// with the confirmation's, and refuses a request whose preview is
    /// stale as `external_change`. A key that was selected, removed or
    /// replaced since the preview is such a change. A key file that has
    /// changed is refused here by the domain's review, as
    /// `external_change` too, with nothing offered; see `refusal`.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let observed = self.observe(call.service)?;
        Ok(Prepared {
            family: Some(OperationFamily::KeyMaterial),
            position: no_position(),
            expected: None,
            observation: Some(observed.to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// In order:
    ///
    /// 1. A row that has completed is a key that was deleted: the domain
    ///    completes a deletion's row in the transaction that removes the
    ///    key's registration and in no other. The deletion is called
    ///    with no review, which the domain answers with "already
    ///    deleted" and without opening the key store: the request is a
    ///    no-op, and is finished.
    /// 2. A row that is pending is a deletion that began. Its own work
    ///    may have removed a file, so what was previewed is not compared,
    ///    and the confirmation is the request's however old it is. A
    ///    fresh review is obtained, and the deletion completes under the
    ///    same operation. A review the domain refuses answers the call
    ///    and leaves the record as it is.
    /// 3. With no row nothing of the request's has been done: the key is
    ///    observed again and compared with what was previewed, and a
    ///    difference is `external_change`.
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

impl Confirmed for KeyDeleteBinding {
    fn row(&mut self, call: &CallContext<'_>, _accepted: &Accepted, journal: &JournalRow) -> Row {
        match journal {
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: _,
                checkpointed: _,
            } => {
                self.review.set(None);
                Row::Recorded
            }
            JournalRow::Pending(PendingOperation::KeyMaterial { phase: _ }) => {
                match self.review(call.service) {
                    Ok(_) => Row::Recorded,
                    Err(answer) => Row::answers(Ran {
                        answer: Answer {
                            local_change: self.removed(Some(journal)),
                            ..answer
                        },
                        standing: Standing::NotRun {
                            journal: journal.clone(),
                        },
                    }),
                }
            }
            JournalRow::Absent => Row::Unrecorded,
            // No row of a deletion reads so. Nothing is run or settled
            // on one that does.
            JournalRow::Pending(
                PendingOperation::Local { state: _, step: _ }
                | PendingOperation::Remote { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => Row::Impossible,
        }
    }

    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        match self.observe(call.service) {
            Ok(found) => Ok(Seen::State { found, own: None }),
            Err(answer) => Err(Ran {
                answer,
                standing: Standing::NotRun {
                    journal: journal.clone(),
                },
            }),
        }
    }

    fn differs(&mut self, journal: JournalRow, _consented: bool) -> Ran {
        self.review.set(None);
        Ran {
            answer: self.answer(ResultCode::ExternalChange),
            standing: Standing::NotRun { journal },
        }
    }

    /// The preview is of the registration alone, read by the preflight:
    /// the key store is not opened for it, and no file is read.
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let preflight = self.preflight(call.service)?;
        let registration = match &preflight {
            GeneratedKeyDeletionPreflight::ConfirmationRequired(registration) => registration,
            GeneratedKeyDeletionPreflight::NotRegistered
            | GeneratedKeyDeletionPreflight::ImportedKey
            | GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared => {
                return Err(self.answer(outcome::key_deletion_preflight_code(&preflight)));
            }
        };
        // A file that is not there is not deleted, and is not listed.
        // One that appears before the request runs is a change to what
        // was observed.
        let mut effects = Vec::new();
        if is_there(&registration.private_key_path) {
            effects.push(PreviewEffect::PrivateKeyFileDeleted);
        }
        if registration
            .public_key_path
            .as_deref()
            .is_some_and(is_there)
        {
            effects.push(PreviewEffect::PublicKeyFileDeleted);
        }
        effects.push(PreviewEffect::KeyUnregistered);
        Ok(Previewed {
            observation: registration_digest(&preflight).to_stored(),
            effects,
            details: PreviewDetailsDto::KeyDeletion(KeyDeletionPreviewDto {
                id: registration.id.to_string(),
                label: registration.label.clone(),
                fingerprint: registration.public_key_fingerprint.clone(),
                private_key_path: registration.private_key_path.to_string_lossy().into_owned(),
                public_key_path: registration
                    .public_key_path
                    .as_deref()
                    .map(|path| path.to_string_lossy().into_owned()),
            }),
        })
    }
}
