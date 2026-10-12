//! `key generate`: a request of the application whose one operation is in
//! the key journal. It writes a key pair into the user's key store and
//! registers it. Every effect is `not_requested`; `data` names the key
//! and, while the key is not registered, where its generation stands.
//!
//! The passphrase of a new key is given to the domain call and to nothing
//! else: every scenario here that protects its key uses the sentinel
//! passphrase, which the world scans every envelope and every request
//! store for.

use manyhands::{
    repository::{
        FailurePoint, KeyGenerationState, KeyMutationDto, MutationDataDto, OperationFamily,
        OperationId, RepositoryService, RequestState, SharedKeyId,
        keys::KeyMaterialPhase,
        request_store::{FinalKind, JournalRow, PendingOperation, ScopeKey},
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode, Scope},
};

use crate::support::{
    FailOnce,
    conformance::{Ask, Row},
    mutation::{
        self, KeyWorld, PASSPHRASE, PASSPHRASE_SENTINEL, REQUEST_1, REQUEST_2, generate_in,
        key_data, recovery_actions,
    },
};

const LABEL: &str = "Work laptop";

/// What every result of a generation holds, whatever its outcome: no
/// effect, and a scope that names nothing.
fn assert_of_the_application(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.command, "key generate");
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.scope, Scope::default());
}

/// Asserts that the request generated the one registered key, and
/// returns the key's ID.
fn assert_generated(world: &KeyWorld, envelope: &Envelope<MutationDataDto>) -> SharedKeyId {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_of_the_application(envelope);
    assert!(envelope.recovery.is_empty());
    let keys = world.service.list_shared_keys().unwrap();
    let [key] = &keys[..] else {
        panic!("one registered key: {keys:?}");
    };
    assert_eq!(
        key_data(envelope),
        &KeyMutationDto {
            id: Some(key.id.to_string()),
            label: Some(LABEL.to_owned()),
            fingerprint: key.public_key_fingerprint.clone(),
            generation: None,
            deletion: None,
        }
    );
    assert!(key.public_key_fingerprint.is_some());
    assert!(!key.selected);
    key.id
}

/// The row of the request's operation in the key journal.
fn journal_row(world: &KeyWorld, envelope: &Envelope<MutationDataDto>) -> JournalRow {
    let operation = envelope.operation_id.as_deref().expect("an operation");
    world
        .service
        .journal_row(
            OperationFamily::KeyMaterial,
            &ScopeKey::application(),
            OperationId::parse(operation).unwrap(),
        )
        .unwrap()
}

const COMPLETED: JournalRow = JournalRow::Final {
    kind: FinalKind::Completed,
    owes_work: false,
    checkpointed: true,
};

const RETAINED: JournalRow = JournalRow::Final {
    kind: FinalKind::RetainedForInspection,
    owes_work: false,
    checkpointed: true,
};

fn state(world: &KeyWorld, request_id: &str) -> Option<RequestState> {
    world.record(request_id).map(|record| record.state)
}

#[test]
fn a_key_is_generated_and_registered() {
    let world = KeyWorld::new();
    let generated = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
    let id = assert_generated(&world, &generated);
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(journal_row(&world, &generated), COMPLETED);

    // The private key is protected by the passphrase the request gave.
    let private = std::fs::read(world.store_directory().join(id.to_string())).unwrap();
    let key = ssh_key::PrivateKey::from_openssh(&private).unwrap();
    assert!(key.is_encrypted());
    assert!(key.decrypt(PASSPHRASE.as_bytes()).is_ok());

    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.scope.as_str(), "application");
    assert_eq!(record.command, "key generate");
    assert_eq!(record.target, "");
    assert_eq!((record.base_ref, record.base_oid), (None, None));
    let [operation] = &record.operations[..] else {
        panic!("one operation: {:?}", record.operations);
    };
    assert_eq!(operation.family, OperationFamily::KeyMaterial);
    assert_eq!(
        Some(operation.operation_id.to_string()),
        generated.operation_id
    );

    // Replay: the stored result. Nothing runs, and no second key is made.
    assert_eq!(
        world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE))),
        generated
    );
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(world.key_operations(), 1);
}

/// The digest a record holds for the same request ID in a world of its
/// own.
fn digest_of(label: &str, passphrase: Option<&str>) -> String {
    let world = KeyWorld::new();
    let generated = world.execute(REQUEST_1, world.generate(label, passphrase));
    assert_eq!(generated.outcome, Outcome::Success);
    world
        .record(REQUEST_1)
        .expect("a record")
        .intent_digest
        .to_string()
}

#[test]
fn the_digest_holds_the_label_and_whether_the_key_is_protected_and_no_passphrase() {
    let protected = digest_of(LABEL, Some(PASSPHRASE));
    assert_eq!(protected, digest_of(LABEL, Some("another passphrase")));
    assert_ne!(protected, digest_of(LABEL, None));
    assert_ne!(protected, digest_of("Another label", Some(PASSPHRASE)));
}

#[test]
fn a_changed_generation_is_rejected_and_another_passphrase_is_the_same_request() {
    let world = KeyWorld::new();
    let generated = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
    let id = assert_generated(&world, &generated);

    // Two requests that differ only in their passphrase are one request:
    // the second is answered with the first's key.
    assert_eq!(
        world.execute(REQUEST_1, world.generate(LABEL, Some("another passphrase"))),
        generated
    );

    for changed in [
        world.generate(LABEL, None),
        world.generate("Another label", Some(PASSPHRASE)),
    ] {
        let mismatch = world.execute(REQUEST_1, changed);
        assert_eq!(
            (mismatch.outcome, mismatch.code, mismatch.failure_class()),
            (
                Outcome::Error,
                ResultCode::RequestMismatch,
                Some(FailureClass::Input)
            )
        );
        assert_of_the_application(&mismatch);
        assert_eq!(mismatch.operation_id, None);
    }
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(world.key_operations(), 1);
}

/// A label that marks the row the generation leaves in the key journal,
/// which stores the label.
const LABEL_SENTINEL: &str = "SENTINEL-LABEL-2e8b";

/// Asserts that no file of the data directory holds the passphrase: the
/// database, its journal and its write-ahead log among them.
fn assert_no_file_holds_the_passphrase(world: &KeyWorld) {
    let needle = PASSPHRASE_SENTINEL.as_bytes();
    for entry in std::fs::read_dir(world.data.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = std::fs::read(&path).unwrap();
            assert!(
                !bytes.windows(needle.len()).any(|window| window == needle),
                "{} holds the passphrase",
                path.display()
            );
        }
    }
}

/// The scan that every scenario relies on reads what a generation leaves:
/// it finds the label there, and does not find the passphrase.
#[test]
fn no_record_journal_row_or_result_of_a_generation_holds_its_passphrase() {
    let world = KeyWorld::new();
    let label = format!("Laptop {LABEL_SENTINEL}");
    let generated = world.execute(REQUEST_1, world.generate(&label, Some(PASSPHRASE)));
    assert_eq!(generated.outcome, Outcome::Success);
    // A real generation, with the passphrase in use.
    let id = key_data(&generated).id.clone().expect("a key");
    let private = std::fs::read(world.store_directory().join(&id)).unwrap();
    assert!(
        ssh_key::PrivateKey::from_openssh(&private)
            .unwrap()
            .decrypt(PASSPHRASE.as_bytes())
            .is_ok()
    );

    // The scan reads what the generation left: it finds the label, which
    // the record stores with the result and the key journal with its row.
    let found = mutation::find_stored_sentinel(world.data.path(), &[LABEL_SENTINEL])
        .expect("the record holds the label");
    assert_eq!(
        (found.table, found.column.as_str()),
        ("request_records", "result_data")
    );
    let journal_label: String = crate::support::items::index(world.data.path())
        .query_row(
            "SELECT generation_label FROM key_material_operations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(journal_label, label);
    // And the passphrase is in none of the stores, nor in any file.
    assert_eq!(
        mutation::find_stored_sentinel(world.data.path(), &[PASSPHRASE_SENTINEL]),
        None
    );
    assert_no_file_holds_the_passphrase(&world);

    // Nor in what is read back of the request.
    let shown = world
        .service
        .show_request(manyhands::repository::RequestId::parse(REQUEST_1).unwrap());
    let text = serde_json::to_string(&shown).unwrap();
    assert!(text.contains(LABEL_SENTINEL), "{text}");
    assert!(!text.contains(PASSPHRASE_SENTINEL), "{text}");
    assert!(
        !serde_json::to_string(&generated)
            .unwrap()
            .contains(PASSPHRASE_SENTINEL)
    );
}

/// The same, for a generation that stopped at each point it can stop at,
/// and for its retry.
#[test]
fn no_store_or_result_of_an_interrupted_generation_holds_its_passphrase() {
    for point in [
        FailurePoint::GenerationEntropyUnavailable,
        FailurePoint::GenerationAfterReservation,
        FailurePoint::GenerationAfterExclusiveCreate,
        FailurePoint::GenerationAfterPrivateWrite,
        FailurePoint::GenerationAfterPublicWrite,
        FailurePoint::GenerationBeforeFinalTransaction,
        FailurePoint::GenerationAfterFinalTransaction,
    ] {
        let mut world = KeyWorld::with_service(|data| FailOnce::at(point).open_service(data));
        // `execute` scans each envelope for the passphrase.
        let stopped = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
        assert_ne!(stopped.outcome, Outcome::Success, "{point:?}");
        assert_eq!(
            mutation::find_stored_sentinel(world.data.path(), &[PASSPHRASE_SENTINEL]),
            None,
            "{point:?}"
        );
        assert_no_file_holds_the_passphrase(&world);
        world.reopen();
        world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
        assert_no_file_holds_the_passphrase(&world);
    }
}

/// Asserts that a generation stopped short of a registered key, in
/// `generation`, and returns the ID of the key it reserved.
fn assert_interrupted(
    envelope: &Envelope<MutationDataDto>,
    generation: KeyGenerationState,
) -> String {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Partial,
            ResultCode::RecoveryRequired,
            Some(FailureClass::Incomplete)
        )
    );
    assert_of_the_application(envelope);
    let data = key_data(envelope);
    assert_eq!(data.label.as_deref(), Some(LABEL));
    assert_eq!(data.fingerprint, None);
    assert_eq!(data.generation, Some(generation));
    data.id.clone().expect("the reserved key")
}

/// A generation that was interrupted once its key pair was written is
/// partial, and its record stays accepted. The same request verifies the
/// pair and registers the key: one key, and one file.
#[test]
fn a_generation_interrupted_after_its_pair_was_written_is_registered_by_its_retry() {
    for point in [
        FailurePoint::GenerationAfterPublicWrite,
        FailurePoint::GenerationBeforeFinalTransaction,
    ] {
        let mut world = KeyWorld::with_service(|data| FailOnce::at(point).open_service(data));
        let stopped = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
        let reserved = assert_interrupted(&stopped, KeyGenerationState::PairWritten);
        assert_eq!(
            journal_row(&world, &stopped),
            JournalRow::Pending(PendingOperation::KeyMaterial {
                phase: KeyMaterialPhase::PairWritten
            }),
            "{point:?}"
        );
        assert_eq!(
            world.private_key_files(),
            std::slice::from_ref(&reserved),
            "{point:?}"
        );
        assert!(world.keys().is_empty(), "{point:?}: not yet registered");
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
        // The same request is what finishes it.
        assert_eq!(recovery_actions(&stopped), ["request.retry"], "{point:?}");
        assert_eq!(stopped.recovery[0].arguments["request_id"], REQUEST_1);

        world.reopen();
        let retry = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
        let id = assert_generated(&world, &retry);
        assert_eq!(id.to_string(), reserved, "{point:?}");
        assert_eq!(retry.operation_id, stopped.operation_id, "{point:?}");
        assert_eq!(world.private_key_files(), [reserved], "{point:?}");
        assert_eq!(world.key_operations(), 1, "{point:?}");
        assert_eq!(journal_row(&world, &retry), COMPLETED, "{point:?}");
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        assert_eq!(
            world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE))),
            retry,
            "{point:?}"
        );
    }
}

/// The key is registered in the transaction that completes the row. A
/// call that failed after it has a registered key and a failure to
/// report; the same request finds the key and returns it.
#[test]
fn a_generation_that_failed_after_registering_its_key_returns_it_on_its_retry() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterFinalTransaction).open_service(data)
    });
    let stopped = world.execute(REQUEST_1, world.generate(LABEL, None));
    let reserved = assert_interrupted(&stopped, KeyGenerationState::Completed);
    assert_eq!(journal_row(&world, &stopped), COMPLETED);
    assert_eq!(world.keys().len(), 1);
    // The row has completed, and the record is not released for that: a
    // record released here would have the next call generate a second key.
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(recovery_actions(&stopped), ["request.retry"]);

    world.reopen();
    let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
    let id = assert_generated(&world, &retry);
    assert_eq!(id.to_string(), reserved);
    assert_eq!(world.private_key_files(), [reserved]);
    assert_eq!(world.key_operations(), 1);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
}

/// A generation that was interrupted before its key pair was complete
/// cannot be continued: the domain retains what it wrote for inspection
/// and makes that final. The retry reports it, and the record is then
/// finished with that result.
#[test]
fn a_generation_interrupted_before_its_pair_was_complete_is_retained_and_finished() {
    for (point, generation, phase, files) in [
        (
            FailurePoint::GenerationAfterReservation,
            KeyGenerationState::Reserved,
            KeyMaterialPhase::Reserved,
            0,
        ),
        (
            FailurePoint::GenerationAfterExclusiveCreate,
            KeyGenerationState::Reserved,
            KeyMaterialPhase::Reserved,
            1,
        ),
        (
            FailurePoint::GenerationAfterPrivateWrite,
            KeyGenerationState::PrivateWritten,
            KeyMaterialPhase::PrivateWritten,
            1,
        ),
    ] {
        let mut world = KeyWorld::with_service(|data| FailOnce::at(point).open_service(data));
        let stopped = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
        let reserved = assert_interrupted(&stopped, generation);
        assert_eq!(
            journal_row(&world, &stopped),
            JournalRow::Pending(PendingOperation::KeyMaterial { phase }),
            "{point:?}"
        );
        let left = world.private_key_files();
        assert_eq!(left.len(), files, "{point:?}");
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
        assert_eq!(recovery_actions(&stopped), ["request.retry"], "{point:?}");

        world.reopen();
        let retry = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
        assert_eq!(
            assert_interrupted(&retry, KeyGenerationState::RetainedForInspection),
            reserved,
            "{point:?}"
        );
        assert_eq!(retry.operation_id, stopped.operation_id, "{point:?}");
        assert_eq!(journal_row(&world, &retry), RETAINED, "{point:?}");
        // Final: nothing continues it, and the same request is not offered.
        assert!(retry.recovery.is_empty(), "{point:?}");
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        // No key is registered, and the files are left where they were.
        assert!(world.keys().is_empty(), "{point:?}");
        assert_eq!(world.private_key_files(), left, "{point:?}");
        assert_eq!(world.key_operations(), 1, "{point:?}");
        assert_eq!(
            world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE))),
            retry,
            "{point:?}"
        );

        // A retained generation blocks no other: a new request generates
        // its own key.
        let other = world.execute(REQUEST_2, world.generate(LABEL, None));
        let id = assert_generated(&world, &other);
        assert_ne!(id.to_string(), reserved, "{point:?}");
    }
}

#[test]
fn a_generation_that_could_not_begin_leaves_nothing() {
    let world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationEntropyUnavailable).open_service(data)
    });
    let failed = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
    assert_eq!(
        (failed.outcome, failed.code, failed.failure_class()),
        (
            Outcome::Error,
            ResultCode::InternalError,
            Some(FailureClass::Internal)
        )
    );
    assert_of_the_application(&failed);
    assert_eq!(key_data(&failed).id, None);
    assert!(failed.recovery.is_empty());
    assert_eq!(world.request_rows(), 0);
    assert_eq!(world.key_operations(), 0);
    assert!(world.private_key_files().is_empty());

    // The same request ID runs at once.
    let generated = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
    assert_generated(&world, &generated);
}

#[test]
fn a_generation_with_no_label_is_refused_before_it_is_accepted() {
    let world = KeyWorld::new();
    let refused = world.execute(REQUEST_1, world.generate("  ", Some(PASSPHRASE)));
    assert_eq!(
        (refused.outcome, refused.code, refused.failure_class()),
        (
            Outcome::Error,
            ResultCode::InvalidInput,
            Some(FailureClass::Input)
        )
    );
    assert_of_the_application(&refused);
    assert_eq!(refused.operation_id, None);
    assert_eq!(world.request_rows(), 0);
    assert_eq!(world.key_operations(), 0);
    assert!(world.private_key_files().is_empty());
}

/// The key was created and the call's result was lost. The same request
/// returns that key: the operation is its own, so the key is the one it
/// generated, and no second file is created.
#[test]
fn a_generation_whose_result_was_lost_returns_the_same_key() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    let lost = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
    assert_eq!(
        (lost.outcome, lost.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    let files = world.private_key_files();
    assert_eq!(files.len(), 1);
    assert_eq!(world.keys().len(), 1);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Accepted);
    let operation = record.operations[0].operation_id.to_string();

    world.reopen();
    let retry = world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE)));
    let id = assert_generated(&world, &retry);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(world.private_key_files(), files);
    assert_eq!(world.key_operations(), 1);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.attempt, 2);
    assert_eq!(
        world.execute(REQUEST_1, world.generate(LABEL, Some(PASSPHRASE))),
        retry
    );
}

#[test]
fn a_generation_that_died_before_its_domain_call_is_made_by_its_retry() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
    });
    let died = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(world.key_operations(), 0);
    assert!(world.private_key_files().is_empty());

    world.reopen();
    let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
    let id = assert_generated(&world, &retry);
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
}

/// The request ID of a generation that wrote its pair and has not ended,
/// used with another input: the result says a change was made that the
/// request ID still owes.
#[test]
fn a_changed_request_under_an_interrupted_generation_s_id_is_partial() {
    let world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterPublicWrite).open_service(data)
    });
    let stopped = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_interrupted(&stopped, KeyGenerationState::PairWritten);

    let mismatch = world.execute(REQUEST_1, world.generate("Another label", None));
    assert_eq!(
        (mismatch.outcome, mismatch.code),
        (Outcome::Partial, ResultCode::RequestMismatch)
    );
    assert_of_the_application(&mismatch);
    assert_eq!(mismatch.operation_id, None);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(world.key_operations(), 1);
}

/// A generation whose row has completed registered its key: the domain
/// completes the row in the transaction that registers the key and in
/// no other. Its lost result is given from the row, whatever has
/// happened to the key since, and the record is finished. Nothing is
/// generated, registered or written again.
#[test]
fn a_completed_generation_s_lost_result_is_given_from_its_row_whatever_became_of_the_key() {
    /// What is done to the key between the lost result and the retry.
    #[derive(Clone, Copy, Debug)]
    enum Since {
        Nothing,
        Removed,
        FilesDeleted,
    }
    for since in [Since::Nothing, Since::Removed, Since::FilesDeleted] {
        let mut world = KeyWorld::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
        });
        let lost = world.execute(REQUEST_1, world.generate(LABEL, None));
        assert_eq!(lost.code, ResultCode::InternalError, "{since:?}");
        let keys = world.service.list_shared_keys().unwrap();
        let [key] = &keys[..] else {
            panic!("one key: {keys:?}");
        };
        let (id, fingerprint) = (key.id, key.public_key_fingerprint.clone());
        assert!(fingerprint.is_some());
        world.reopen();
        match since {
            Since::Nothing => {}
            Since::Removed => {
                world.service.unregister_shared_key(id).unwrap();
            }
            Since::FilesDeleted => {
                std::fs::remove_file(world.store_directory().join(id.to_string())).unwrap();
                std::fs::remove_file(world.store_directory().join(format!("{id}.pub"))).unwrap();
            }
        }
        let files = world.private_key_files();
        let registered = world.keys();

        let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
        assert_eq!(
            (retry.outcome, retry.code, retry.failure_class()),
            (Outcome::Success, ResultCode::Ok, None),
            "{since:?}"
        );
        assert_of_the_application(&retry);
        assert!(retry.recovery.is_empty(), "{since:?}");
        assert_eq!(
            key_data(&retry),
            &KeyMutationDto {
                id: Some(id.to_string()),
                label: Some(LABEL.to_owned()),
                fingerprint: fingerprint.clone(),
                generation: None,
                deletion: None,
            },
            "{since:?}"
        );
        assert_eq!(journal_row(&world, &retry), COMPLETED, "{since:?}");
        // Nothing was generated, registered or written for it.
        assert_eq!(world.private_key_files(), files, "{since:?}");
        assert_eq!(world.keys(), registered, "{since:?}");
        assert_eq!(world.key_operations(), 1, "{since:?}");
        let record = world.record(REQUEST_1).expect("a record");
        assert_eq!(
            (record.state, record.attempt),
            (RequestState::Finished, 2),
            "{since:?}"
        );
        assert_eq!(
            world.execute(REQUEST_1, world.generate(LABEL, None)),
            retry,
            "{since:?}"
        );
    }
}

/// A record of a generation holds its operation. One that has lost it is
/// not a record any call wrote: it is not entered again under another
/// operation ID, which would generate a second key, and it is not
/// replayed.
#[test]
fn a_generation_s_record_that_lost_its_operation_is_not_run_or_replayed() {
    let forget_the_operation = |world: &KeyWorld| {
        let removed = crate::support::items::index(world.data.path())
            .execute("DELETE FROM request_operations", [])
            .unwrap();
        assert_eq!(removed, 1);
    };

    // An accepted record.
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    world.execute(REQUEST_1, world.generate(LABEL, None));
    let files = world.private_key_files();
    assert_eq!(files.len(), 1);
    world.reopen();
    forget_the_operation(&world);
    let entered = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(
        (entered.outcome, entered.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(entered.operation_id, None);
    assert_eq!(world.private_key_files(), files);
    assert_eq!(world.key_operations(), 1);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));

    // A finished one.
    let world = KeyWorld::new();
    let generated = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_generated(&world, &generated);
    forget_the_operation(&world);
    let replayed = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(replayed.code, ResultCode::InternalError);
    assert_eq!(world.key_operations(), 1);
}

/// Has a second call of `REQUEST_1` run whole, on a service `open` gives,
/// while the world's next call of it sits between its acceptance and its
/// domain call. What the second call returned is left in the slot.
fn run_beside(
    world: &KeyWorld,
    open: impl FnOnce(&std::path::Path) -> RepositoryService + Send + 'static,
) -> std::sync::Arc<std::sync::Mutex<Option<Envelope<MutationDataDto>>>> {
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let (data, home) = (world.data.path().to_owned(), world.home.path().to_owned());
    let returned = slot.clone();
    world.service.set_request_hook_for_testing(move || {
        let beside =
            mutation::execute_with(&open(&data), REQUEST_1, generate_in(&home, LABEL, None));
        *returned.lock().unwrap() = Some(beside);
    });
    slot
}

/// The same request arrives twice at once: one key, and both callers are
/// given it.
#[test]
fn two_calls_of_one_generation_at_once_make_one_key() {
    let world = KeyWorld::new();
    let beside = run_beside(&world, |data| RepositoryService::open_at(data).unwrap());

    let first = world.execute(REQUEST_1, world.generate(LABEL, None));
    let id = assert_generated(&world, &first);
    let beside = beside.lock().unwrap().take().expect("the second call ran");
    assert_eq!(assert_generated(&world, &beside), id);
    assert_eq!(beside.operation_id, first.operation_id);
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(world.key_operations(), 1);
    // The second call entered the record and settled it.
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!((record.state, record.attempt), (RequestState::Finished, 2));
}

/// The second of two calls at once fails before the generation has a
/// row. It cannot know that the call it entered beside has stopped, so
/// it does not release the record: released, the first call's key would
/// belong to no record, and the second caller's retry would be a new
/// request and generate another.
#[test]
fn a_call_that_fails_beside_a_generation_that_has_not_begun_does_not_release_its_record() {
    let mut world = KeyWorld::new();
    let beside = run_beside(&world, |data| {
        FailOnce::at(FailurePoint::GenerationEntropyUnavailable).open_service(data)
    });

    let first = world.execute(REQUEST_1, world.generate(LABEL, None));
    let id = assert_generated(&world, &first);
    let beside = beside.lock().unwrap().take().expect("the second call ran");
    assert_eq!(
        (beside.outcome, beside.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_of_the_application(&beside);
    assert_eq!(beside.operation_id, first.operation_id);
    // The record is still there, and the same request is what settles it.
    assert_eq!(recovery_actions(&beside), ["request.retry"]);
    let record = world.record(REQUEST_1).expect("the record is kept");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));

    // The second caller sends its request again and is given the one key.
    world.reopen();
    let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(assert_generated(&world, &retry), id);
    assert_eq!(retry.operation_id, first.operation_id);
    assert_eq!(world.private_key_files(), [id.to_string()]);
    assert_eq!(world.key_operations(), 1);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
}

/// The key store is locked by another holder while a generation whose
/// pair is written is retried. The call stops as busy with the pair
/// still written and its row pending, so the record stays accepted, and
/// the retry after the lock is released registers the one key.
#[test]
fn a_retry_that_finds_the_key_store_busy_keeps_its_record_and_is_finished_later() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterPublicWrite).open_service(data)
    });
    let stopped = world.execute(REQUEST_1, world.generate(LABEL, None));
    let reserved = assert_interrupted(&stopped, KeyGenerationState::PairWritten);
    world.reopen();

    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(world.store_directory().join(".lock"))
        .unwrap();
    assert!(fs4::fs_std::FileExt::try_lock_exclusive(&lock).unwrap());
    let busy = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(
        (busy.outcome, busy.code),
        (Outcome::Partial, ResultCode::Busy)
    );
    assert_of_the_application(&busy);
    assert_eq!(busy.operation_id, stopped.operation_id);
    assert_eq!(key_data(&busy).id, None);
    assert_eq!(recovery_actions(&busy), ["request.retry"]);
    assert_eq!(
        journal_row(&world, &busy),
        JournalRow::Pending(PendingOperation::KeyMaterial {
            phase: KeyMaterialPhase::PairWritten
        })
    );
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
    assert!(world.keys().is_empty());
    fs4::fs_std::FileExt::unlock(&lock).unwrap();
    drop(lock);

    let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(assert_generated(&world, &retry).to_string(), reserved);
    assert_eq!(world.private_key_files(), [reserved]);
    assert_eq!(world.key_operations(), 1);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
}

/// The key store is no part of a request's identity: the digest holds the
/// label and the protection. A retry that names another store is the same
/// request, and the domain refuses to take the operation up there. A
/// limit, pinned: the answer is `internal_error`, nothing is created in
/// either store, and the record is kept for the retry that names the
/// store the generation began in.
#[test]
fn a_retry_with_another_key_store_is_refused_and_creates_nothing() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterPublicWrite).open_service(data)
    });
    let stopped = world.execute(REQUEST_1, world.generate(LABEL, None));
    let reserved = assert_interrupted(&stopped, KeyGenerationState::PairWritten);
    world.reopen();

    let elsewhere = tempfile::tempdir().unwrap();
    let refused = world.execute(REQUEST_1, generate_in(elsewhere.path(), LABEL, None));
    assert_eq!(
        (refused.outcome, refused.code),
        (Outcome::Partial, ResultCode::InternalError)
    );
    assert_of_the_application(&refused);
    assert_eq!(refused.operation_id, stopped.operation_id);
    assert!(!elsewhere.path().join(".ssh").exists());
    assert_eq!(world.private_key_files(), std::slice::from_ref(&reserved));
    assert!(world.keys().is_empty());
    assert_eq!(world.key_operations(), 1);
    assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));

    let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(assert_generated(&world, &retry).to_string(), reserved);
    assert_eq!(world.key_operations(), 1);
}

/// The row of `key generate` in the conformance table.
pub(crate) const CONFORMANCE: [Row; 1] =
    [Row::new("key generate", false, "invalid_input", |rule| {
        let world = KeyWorld::new();
        mutation::drive_plain(
            &world.service,
            world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Unconsented => world.generate(LABEL, Some(PASSPHRASE)),
                Ask::Changed => world.generate("Another label", Some(PASSPHRASE)),
                // No label.
                Ask::Refused => world.generate("  ", Some(PASSPHRASE)),
            },
            rule,
        );
        Ok(())
    })];
