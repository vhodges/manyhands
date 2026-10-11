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
        OperationId, RequestState, SharedKeyId,
        keys::KeyMaterialPhase,
        request_store::{FinalKind, JournalRow, PendingOperation, ScopeKey},
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode, Scope},
};

use crate::support::{
    FailOnce,
    mutation::{
        self, KeyWorld, PASSPHRASE, PASSPHRASE_SENTINEL, REQUEST_1, REQUEST_2, key_data,
        recovery_actions,
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

/// A limit, pinned: the key a generation registered is removed before
/// the generation's lost result is asked for again. The domain no longer
/// finds the registration and says so. The record is kept: released, the
/// next call would be a new request and would generate a second key for
/// the same request ID.
#[test]
fn a_generated_key_removed_before_its_lost_result_is_asked_for_is_not_generated_again() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    let lost = world.execute(REQUEST_1, world.generate(LABEL, None));
    assert_eq!(lost.code, ResultCode::InternalError);
    let [(id, false)] = world.keys()[..] else {
        panic!("one key: {:?}", world.keys());
    };
    world.reopen();
    world.service.unregister_shared_key(id).unwrap();

    for _ in 0..2 {
        let retry = world.execute(REQUEST_1, world.generate(LABEL, None));
        assert_eq!(
            (retry.outcome, retry.code),
            (Outcome::Error, ResultCode::KeyNotFound)
        );
        assert_of_the_application(&retry);
        assert_eq!(recovery_actions(&retry), ["request.retry"]);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
        assert!(world.keys().is_empty());
        assert_eq!(world.private_key_files(), [id.to_string()]);
        assert_eq!(world.key_operations(), 1);
    }
}
