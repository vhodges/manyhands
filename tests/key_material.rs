use manyhands::repository::keys::{
    GenerateSharedKeyOutcome, GenerateSharedKeyRequest, KeyMaterialErrorKind, KeyMaterialPhase,
    KeyProtection, KeyStore, RecoveryAction, SecretPassphrase,
};
use manyhands::repository::{FailurePoint, OperationId, REGISTRY_FILE, RepositoryService};
use std::fs;

fn request(id: OperationId, protection: KeyProtection) -> GenerateSharedKeyRequest {
    GenerateSharedKeyRequest {
        operation_id: id,
        label: "test label".into(),
        protection,
    }
}
fn created(outcome: GenerateSharedKeyOutcome) -> manyhands::repository::SharedKeyRegistration {
    match outcome {
        GenerateSharedKeyOutcome::Created(r) | GenerateSharedKeyOutcome::AlreadyCreated(r) => r,
        _ => panic!("expected creation"),
    }
}
#[test]
fn generation_plain_and_encrypted_round_trip() {
    for encrypted in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let password = format!("passphrase-{}", OperationId::new());
        let protection = if encrypted {
            KeyProtection::Passphrase(SecretPassphrase::new(password.clone()).unwrap())
        } else {
            KeyProtection::Unencrypted
        };
        let r = created(
            service
                .generate_shared_key(&store, request(OperationId::new(), protection))
                .unwrap(),
        );
        assert!(!r.selected);
        assert_eq!(
            r.private_key_path,
            home.path().join(".ssh/manyhands").join(r.id.to_string())
        );
        assert_eq!(
            r.public_key_path.as_ref().unwrap(),
            &r.private_key_path.with_extension("pub")
        );
        let bytes = fs::read(&r.private_key_path).unwrap();
        let key = ssh_key::PrivateKey::from_openssh(&bytes).unwrap();
        let public = ssh_key::PublicKey::from_openssh(
            &fs::read_to_string(r.public_key_path.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(key.algorithm(), ssh_key::Algorithm::Ed25519);
        assert!(key.comment().is_empty() && public.comment().is_empty());
        assert!(key.public_key() == &public);
        assert_eq!(
            r.public_key_fingerprint,
            Some(public.fingerprint(ssh_key::HashAlg::Sha256).to_string())
        );
        assert_eq!(key.is_encrypted(), encrypted);
        if encrypted {
            assert_eq!(key.cipher(), ssh_key::Cipher::Aes256Ctr);
            assert!(matches!(key.kdf(), ssh_key::Kdf::Bcrypt { rounds: 16, .. }));
            assert!(key.decrypt(password.as_bytes()).is_ok());
            assert!(key.decrypt(b"incorrect").is_err());
        }
        for entry in fs::read_dir(data.path()).unwrap().flatten() {
            if entry.path().is_file() {
                let persisted = fs::read(entry.path()).unwrap();
                assert!(
                    !persisted
                        .windows(password.len())
                        .any(|w| w == password.as_bytes())
                );
                assert!(!persisted.windows(bytes.len()).any(|w| w == bytes));
            }
        }
    }
}
#[test]
fn generation_rejects_empty_or_nul_passphrase() {
    assert!(SecretPassphrase::new(String::new()).is_err());
    assert!(SecretPassphrase::new("bad\0value".into()).is_err());
}
#[test]
fn generation_preserves_passphrase_bytes() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let pass = "  雪\t\n ";
    let secret = SecretPassphrase::new(pass.into()).unwrap();
    assert!(!format!("{secret:?}").contains(pass));
    let r = created(
        service
            .generate_shared_key(
                &store,
                request(OperationId::new(), KeyProtection::Passphrase(secret)),
            )
            .unwrap(),
    );
    let key = ssh_key::PrivateKey::from_openssh(fs::read(r.private_key_path).unwrap()).unwrap();
    assert!(key.decrypt(pass.as_bytes()).is_ok());
    assert!(key.decrypt(pass.trim().as_bytes()).is_err());
}
#[test]
fn generation_replay_never_creates_a_second_pair() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let id = OperationId::new();
    let r = created(
        service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap(),
    );
    let original = fs::read(&r.private_key_path).unwrap();
    let replay = service
        .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
        .unwrap();
    assert!(matches!(replay, GenerateSharedKeyOutcome::AlreadyCreated(ref v) if v.id == r.id));
    assert!(fs::read(&r.private_key_path).unwrap() == original);
    assert_eq!(service.list_shared_keys().unwrap().len(), 1);
    fs::remove_file(&r.private_key_path).unwrap();
    assert!(matches!(
        service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap(),
        GenerateSharedKeyOutcome::RecoveryRequired(_)
    ));
    assert!(!r.private_key_path.exists());
}
#[test]
fn generation_interruption_recovery_boundaries() {
    for (point, phase, retry_completes) in [
        (
            FailurePoint::GenerationAfterReservation,
            KeyMaterialPhase::Reserved,
            false,
        ),
        (
            FailurePoint::GenerationAfterExclusiveCreate,
            KeyMaterialPhase::Reserved,
            false,
        ),
        (
            FailurePoint::GenerationAfterPrivateWrite,
            KeyMaterialPhase::PrivateWritten,
            false,
        ),
        (
            FailurePoint::GenerationAfterPublicWrite,
            KeyMaterialPhase::PairWritten,
            true,
        ),
        (
            FailurePoint::GenerationBeforeFinalTransaction,
            KeyMaterialPhase::PairWritten,
            true,
        ),
    ] {
        let home = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service =
            RepositoryService::open_at_with_failure_point_for_testing(data.path(), point).unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let id = OperationId::new();
        let outcome = service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap();
        let GenerateSharedKeyOutcome::RecoveryRequired(recovery) = outcome else {
            panic!("expected interruption")
        };
        assert_eq!(recovery.phase, phase);
        assert!(service.list_shared_keys().unwrap().is_empty());
        assert_eq!(service.list_key_material_recovery().unwrap().len(), 1);
        let retried = service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap();
        if retry_completes {
            let r = created(retried);
            assert_eq!(r.id, recovery.key_id);
            assert_eq!(service.list_shared_keys().unwrap().len(), 1);
        } else {
            assert!(
                matches!(retried, GenerateSharedKeyOutcome::RecoveryRequired(ref r) if r.phase == KeyMaterialPhase::RetainedForInspection && r.recovery_action == RecoveryAction::InspectRetainedFiles)
            );
            assert!(service.list_shared_keys().unwrap().is_empty());
        }
    }
}
#[test]
fn generation_randomness_failure_is_typed_and_creates_nothing() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        data.path(),
        FailurePoint::GenerationEntropyUnavailable,
    )
    .unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let e = service
        .generate_shared_key(
            &store,
            request(OperationId::new(), KeyProtection::Unencrypted),
        )
        .unwrap_err();
    assert_eq!(e.kind, KeyMaterialErrorKind::RandomnessUnavailable);
    assert!(!home.path().join(".ssh").exists());
    assert!(service.list_shared_keys().unwrap().is_empty());
    let c = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM key_material_operations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn generation_reservation_identity_is_registration_identity() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let id = OperationId::new();
    let r = created(
        service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap(),
    );
    let c = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let (journal,evidence): (String,String) = c.query_row("SELECT o.key_id,e.key_id FROM key_material_operations o JOIN owned_generated_keys e ON e.key_id=o.key_id WHERE o.operation_id=?1", [id.to_string()], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
    assert_eq!(journal, r.id.to_string());
    assert_eq!(evidence, r.id.to_string());
}
#[test]
fn generation_registry_failure_preserves_protected_files() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let id = OperationId::new();
    let c = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    c.execute_batch("CREATE TRIGGER reject_registration BEFORE INSERT ON shared_ssh_keys BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
    let GenerateSharedKeyOutcome::RecoveryRequired(recovery) = service
        .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
        .unwrap()
    else {
        panic!("expected recovery")
    };
    assert_eq!(recovery.phase, KeyMaterialPhase::PairWritten);
    assert_eq!(
        recovery.failure_code.unwrap().as_str(),
        "registry-unavailable"
    );
    let path = home
        .path()
        .join(".ssh/manyhands")
        .join(recovery.key_id.to_string());
    let original = fs::read(&path).unwrap();
    assert!(service.list_shared_keys().unwrap().is_empty());
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM owned_generated_keys", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    c.execute_batch("DROP TRIGGER reject_registration").unwrap();
    let r = created(
        service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap(),
    );
    assert_eq!(r.id, recovery.key_id);
    assert!(fs::read(path).unwrap() == original);
}
#[test]
fn generation_crash_gap_does_not_adopt_unproven_files() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        data.path(),
        FailurePoint::GenerationAfterExclusiveCreate,
    )
    .unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let id = OperationId::new();
    let GenerateSharedKeyOutcome::RecoveryRequired(recovery) = service
        .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
        .unwrap()
    else {
        panic!("expected recovery")
    };
    let path = home
        .path()
        .join(".ssh/manyhands")
        .join(recovery.key_id.to_string());
    fs::write(&path, b"collision marker").unwrap();
    let outcome = service
        .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
        .unwrap();
    assert!(
        matches!(outcome,GenerateSharedKeyOutcome::RecoveryRequired(r) if r.phase == KeyMaterialPhase::RetainedForInspection)
    );
    assert!(fs::read(&path).unwrap() == b"collision marker");
    assert!(service.list_shared_keys().unwrap().is_empty());
}
#[test]
fn generation_completed_retry_does_not_restore_unregistered_key() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        data.path(),
        FailurePoint::GenerationAfterFinalTransaction,
    )
    .unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let id = OperationId::new();
    assert!(
        matches!(service.generate_shared_key(&store, request(id,KeyProtection::Unencrypted)).unwrap(),GenerateSharedKeyOutcome::RecoveryRequired(r) if r.phase == KeyMaterialPhase::Completed)
    );
    let r = created(
        service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap(),
    );
    assert!(service.list_key_material_recovery().unwrap().is_empty());
    service.unregister_shared_key(r.id).unwrap();
    let e = service
        .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
        .unwrap_err();
    assert_eq!(e.kind, KeyMaterialErrorKind::NotRegistered);
    assert!(r.private_key_path.is_file());
    assert!(service.list_shared_keys().unwrap().is_empty());
}
#[test]
fn generation_reused_operation_rejects_different_label_and_store() {
    let home = tempfile::tempdir().unwrap();
    let other_home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let id = OperationId::new();
    let r = created(
        service
            .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
            .unwrap(),
    );
    let original = fs::read(&r.private_key_path).unwrap();
    let mut different = request(id, KeyProtection::Unencrypted);
    different.label = "different".into();
    assert_eq!(
        service
            .generate_shared_key(&store, different)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OperationMismatch
    );
    let other_store = KeyStore::for_home(other_home.path()).unwrap();
    assert_eq!(
        service
            .generate_shared_key(&other_store, request(id, KeyProtection::Unencrypted))
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OperationMismatch
    );
    assert!(!other_home.path().join(".ssh").exists());
    assert!(fs::read(&r.private_key_path).unwrap() == original);
}
