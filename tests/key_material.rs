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
    let GenerateSharedKeyOutcome::RecoveryRequired(recovery) = service
        .generate_shared_key(&store, request(id, KeyProtection::Unencrypted))
        .unwrap()
    else {
        panic!("missing source must require inspection")
    };
    assert_eq!(recovery.phase, KeyMaterialPhase::Completed);
    assert_eq!(
        recovery.recovery_action,
        RecoveryAction::InspectRetainedFiles
    );
    assert_eq!(recovery.failure_code.unwrap().as_str(), "source-missing");
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
        assert_eq!(recovery.recovery_action, RecoveryAction::RetryGeneration);
        let expected_code = if point == FailurePoint::GenerationBeforeFinalTransaction {
            "registry-unavailable"
        } else {
            "storage-unavailable"
        };
        assert_eq!(
            recovery.failure_code.as_ref().unwrap().as_str(),
            expected_code
        );
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
                matches!(retried, GenerateSharedKeyOutcome::RecoveryRequired(ref r) if r.phase == KeyMaterialPhase::RetainedForInspection && r.recovery_action == RecoveryAction::InspectRetainedFiles && r.failure_code.as_ref().unwrap().as_str() == expected_code)
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
        matches!(outcome,GenerateSharedKeyOutcome::RecoveryRequired(r) if r.phase == KeyMaterialPhase::RetainedForInspection && r.recovery_action == RecoveryAction::InspectRetainedFiles && r.failure_code.as_ref().unwrap().as_str() == "storage-unavailable")
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
        matches!(service.generate_shared_key(&store, request(id,KeyProtection::Unencrypted)).unwrap(),GenerateSharedKeyOutcome::RecoveryRequired(r) if r.phase == KeyMaterialPhase::Completed && r.recovery_action == RecoveryAction::InspectRetainedFiles && r.failure_code.as_ref().unwrap().as_str() == "registry-unavailable")
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

use manyhands::repository::keys::{
    GeneratedKeyUnlockOutcome, KeyMaterialAction, PassphraseResponse, RegisterSharedKeyOutcome,
    RegisterSharedKeyRequest, SelectedKeyInspection, SessionCredentialProvider, SessionCredentials,
    SharedKeyOwnership, SharedKeyRegistration, UnlockRequest,
};
use std::{cell::Cell, collections::VecDeque, rc::Rc};

struct UnlockProvider {
    calls: Rc<Cell<usize>>,
    responses: VecDeque<PassphraseResponse>,
    hook: Option<Box<dyn FnOnce()>>,
}
impl SessionCredentialProvider for UnlockProvider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        self.calls.set(self.calls.get() + 1);
        if let Some(hook) = self.hook.take() {
            hook();
        }
        self.responses
            .pop_front()
            .expect("unexpected passphrase prompt")
    }
}
fn supplied(value: &str) -> PassphraseResponse {
    PassphraseResponse::Supplied(SecretPassphrase::new(value.to_owned()).unwrap())
}
fn credentials(
    responses: impl IntoIterator<Item = PassphraseResponse>,
) -> (SessionCredentials<UnlockProvider>, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    (
        SessionCredentials::new(UnlockProvider {
            calls: calls.clone(),
            responses: responses.into_iter().collect(),
            hook: None,
        }),
        calls,
    )
}
struct SelectedFixture {
    _home: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    store: KeyStore,
    registration: SharedKeyRegistration,
    password: String,
}
impl SelectedFixture {
    fn new(encrypted: bool) -> Self {
        let home = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let password = format!("unlock-{}", OperationId::new());
        let protection = if encrypted {
            KeyProtection::Passphrase(SecretPassphrase::new(password.clone()).unwrap())
        } else {
            KeyProtection::Unencrypted
        };
        let mut registration = created(
            service
                .generate_shared_key(&store, request(OperationId::new(), protection))
                .unwrap(),
        );
        service.select_shared_key(registration.id).unwrap();
        registration.selected = true;
        Self {
            _home: home,
            data,
            service,
            store,
            registration,
            password,
        }
    }
}
fn import_selected(service: &RepositoryService, path: &std::path::Path) -> SharedKeyRegistration {
    let RegisterSharedKeyOutcome::Registered(mut r) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "external".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: path.to_owned(),
            public_key_path: None,
        })
        .unwrap()
    else {
        panic!("expected registration")
    };
    service.select_shared_key(r.id).unwrap();
    r.selected = true;
    r
}
#[test]
fn import_readability_does_not_parse_private_material() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    assert_eq!(
        service.inspect_selected_key(&store).unwrap(),
        SelectedKeyInspection::NoSelection
    );
    let path = home.path().join("not-a-key");
    let bytes = b"deliberately not any private key encoding";
    fs::write(&path, bytes).unwrap();
    let r = import_selected(&service, &path);
    assert!(
        matches!(service.inspect_selected_key(&store).unwrap(), SelectedKeyInspection::ImportedReadable { registration, .. } if registration == r)
    );
    let (mut session, calls) = credentials([]);
    assert_eq!(
        service.unlock_generated_key(&store, &mut session).unwrap(),
        GeneratedKeyUnlockOutcome::ImportedValidationDeferred(r.clone())
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(service.list_shared_keys().unwrap(), vec![r]);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(!home.path().join(".ssh").exists());
}
#[test]
fn import_missing_and_directory_retain_registration() {
    for directory in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let path = home.path().join("source");
        if directory {
            fs::create_dir(&path).unwrap();
        }
        let r = import_selected(&service, &path);
        let e = service.inspect_selected_key(&store).unwrap_err();
        assert_eq!(
            e.kind,
            if directory {
                KeyMaterialErrorKind::NotRegularFile
            } else {
                KeyMaterialErrorKind::SourceMissing
            }
        );
        assert_eq!(e.operation, KeyMaterialAction::Inspect);
        assert_eq!(e.key_id, Some(r.id));
        assert_eq!(service.list_shared_keys().unwrap(), vec![r]);
    }
}
#[cfg(unix)]
#[test]
fn import_symlink_and_inaccessible_source() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let path = home.path().join("source");
    let link = home.path().join("link");
    fs::write(&path, b"opaque external bytes").unwrap();
    symlink(&path, &link).unwrap();
    let r = import_selected(&service, &link);
    assert!(matches!(
        service.inspect_selected_key(&store).unwrap(),
        SelectedKeyInspection::ImportedReadable { .. }
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    // A root test process can read mode-000 sources by definition.
    if unsafe { libc::geteuid() } != 0 {
        assert_eq!(
            service.inspect_selected_key(&store).unwrap_err().kind,
            KeyMaterialErrorKind::SourceUnreadable
        );
    }
    assert_eq!(service.list_shared_keys().unwrap(), vec![r]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
}
#[cfg(unix)]
#[test]
fn import_fifo_is_rejected_without_blocking() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    const CHILD: &str = "MANYHANDS_IMPORT_FIFO_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let home = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let path = home.path().join("fifo");
        let cpath = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        import_selected(&service, &path);
        assert_eq!(
            service.inspect_selected_key(&store).unwrap_err().kind,
            KeyMaterialErrorKind::NotRegularFile
        );
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "import_fifo_is_rejected_without_blocking"])
        .env(CHILD, "1")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("FIFO inspection blocked");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn generated_unlock_round_trip_reuses_session() {
    let f = SelectedFixture::new(true);
    assert!(
        matches!(f.service.inspect_selected_key(&f.store).unwrap(), SelectedKeyInspection::Generated { registration, .. } if registration == f.registration)
    );
    let (mut session, calls) = credentials([supplied(&f.password)]);
    for _ in 0..2 {
        assert_eq!(
            f.service
                .unlock_generated_key(&f.store, &mut session)
                .unwrap(),
            GeneratedKeyUnlockOutcome::Ready(f.registration.clone())
        );
    }
    assert_eq!(calls.get(), 1);
    session.clear();
    let (mut other, other_calls) = credentials([supplied(&f.password)]);
    assert!(matches!(
        f.service
            .unlock_generated_key(&f.store, &mut other)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(_)
    ));
    assert_eq!(other_calls.get(), 1);
}
#[test]
fn unencrypted_selected_key_does_not_prompt() {
    let f = SelectedFixture::new(false);
    let (mut session, calls) = credentials([]);
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(f.registration)
    );
    assert_eq!(calls.get(), 0);
    f.service.clear_shared_key_selection().unwrap();
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::NoSelection
    );
}
#[test]
fn generated_unlock_cancel_preserves_state() {
    let f = SelectedFixture::new(true);
    let original = fs::read(&f.registration.private_key_path).unwrap();
    let (mut session, calls) = credentials([
        PassphraseResponse::Cancelled,
        PassphraseResponse::Unavailable,
        supplied(&f.password),
    ]);
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Cancelled
    );
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::ProviderUnavailable
    );
    assert!(matches!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(_)
    ));
    assert_eq!(calls.get(), 3);
    assert_eq!(
        f.service.list_shared_keys().unwrap(),
        vec![f.registration.clone()]
    );
    assert!(
        fs::read(&f.registration.private_key_path).unwrap() == original,
        "unlock changed the private key file"
    );
    assert!(f.service.list_key_material_recovery().unwrap().is_empty());
}
#[test]
fn generated_unlock_wrong_passphrase_preserves_registration() {
    let f = SelectedFixture::new(true);
    let (mut session, calls) = credentials([supplied("wrong"), supplied(&f.password)]);
    let e = f
        .service
        .unlock_generated_key(&f.store, &mut session)
        .unwrap_err();
    assert_eq!(e.kind, KeyMaterialErrorKind::UnlockFailed);
    assert_eq!(e.operation, KeyMaterialAction::Unlock);
    assert_eq!(e.key_id, Some(f.registration.id));
    assert!(!format!("{e:?}").contains(&f.password));
    assert_eq!(
        f.service.list_shared_keys().unwrap(),
        vec![f.registration.clone()]
    );
    assert!(matches!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(_)
    ));
    assert_eq!(calls.get(), 2);
}
#[test]
fn selected_key_changes_during_prompt() {
    let f = SelectedFixture::new(true);
    let other = RepositoryService::open_at(f.data.path()).unwrap();
    let calls = Rc::new(Cell::new(0));
    let mut session = SessionCredentials::new(UnlockProvider {
        calls: calls.clone(),
        responses: [supplied(&f.password), supplied(&f.password)].into(),
        hook: Some(Box::new(move || {
            other.clear_shared_key_selection().unwrap();
        })),
    });
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::SelectionChanged
    );
    f.service.select_shared_key(f.registration.id).unwrap();
    assert!(matches!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(_)
    ));
    assert_eq!(calls.get(), 2);
}
#[test]
fn key_source_changes_during_prompt() {
    let f = SelectedFixture::new(true);
    let path = f.registration.private_key_path.clone();
    let original = fs::read(&path).unwrap();
    let store = f.store.clone();
    let other = RepositoryService::open_at(f.data.path()).unwrap();
    let calls = Rc::new(Cell::new(0));
    let mut session = SessionCredentials::new(UnlockProvider {
        calls: calls.clone(),
        responses: [supplied(&f.password), supplied(&f.password)].into(),
        hook: Some(Box::new(move || {
            // Generating through a second service proves the store lock is released.
            other
                .generate_shared_key(
                    &store,
                    request(OperationId::new(), KeyProtection::Unencrypted),
                )
                .unwrap();
            fs::write(path, b"changed while prompting").unwrap();
        })),
    });
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::SourceChanged
    );
    fs::write(&f.registration.private_key_path, original).unwrap();
    assert!(matches!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(_)
    ));
    assert_eq!(calls.get(), 2);
}
#[test]
fn generated_unlock_bounds_file_and_kdf_work() {
    let f = SelectedFixture::new(false);
    let original = fs::read(&f.registration.private_key_path).unwrap();
    let key = ssh_key::PrivateKey::from_openssh(&original).unwrap();
    let mut invalid = vec![vec![b'x'; 65537], b"malformed sensitive material".to_vec()];
    for (cipher, rounds) in [
        (ssh_key::Cipher::Aes256Cbc, 16),
        (ssh_key::Cipher::Aes256Ctr, 1),
    ] {
        invalid.push(
            key.encrypt_with(
                cipher,
                ssh_key::Kdf::Bcrypt {
                    salt: vec![1; 16],
                    rounds,
                },
                42,
                b"test",
            )
            .unwrap()
            .to_openssh(ssh_key::LineEnding::LF)
            .unwrap()
            .as_bytes()
            .to_vec(),
        );
    }
    // Modify the serialized work factor without performing that expensive KDF.
    let encrypted = ssh_key::PrivateKey::from_openssh(invalid.last().unwrap()).unwrap();
    let mut encoded = encrypted.to_bytes().unwrap();
    let rounds_offset = encoded.windows(16).position(|v| v == [1; 16]).unwrap() + 16;
    encoded[rounds_offset..rounds_offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    invalid.push(
        ssh_key::PrivateKey::from_bytes(&encoded)
            .unwrap()
            .to_openssh(ssh_key::LineEnding::LF)
            .unwrap()
            .as_bytes()
            .to_vec(),
    );
    let (mut session, calls) = credentials([]);
    for bytes in invalid {
        fs::write(&f.registration.private_key_path, bytes).unwrap();
        let e = f
            .service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap_err();
        assert_eq!(e.kind, KeyMaterialErrorKind::InvalidGeneratedKey);
        assert_eq!(e.operation, KeyMaterialAction::Unlock);
        assert!(!format!("{e:?}").contains("sensitive"));
    }
    assert_eq!(calls.get(), 0);
}
#[test]
fn generated_unlock_checks_public_identity() {
    let f = SelectedFixture::new(false);
    let (mut session, calls) = credentials([]);
    assert!(matches!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap(),
        GeneratedKeyUnlockOutcome::Ready(_)
    ));
    let replacement = created(
        f.service
            .generate_shared_key(
                &f.store,
                request(OperationId::new(), KeyProtection::Unencrypted),
            )
            .unwrap(),
    );
    fs::write(
        &f.registration.private_key_path,
        fs::read(replacement.private_key_path).unwrap(),
    )
    .unwrap();
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::InvalidGeneratedKey
    );
    assert_eq!(calls.get(), 0);
}
#[test]
fn generated_inspection_requires_creation_evidence_and_expected_paths() {
    let f = SelectedFixture::new(false);
    let other_home = tempfile::tempdir().unwrap();
    let other_store = KeyStore::for_home(other_home.path()).unwrap();
    assert_eq!(
        f.service
            .inspect_selected_key(&other_store)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::UnsafePath
    );
    assert!(!other_home.path().join(".ssh").exists());
    let c = rusqlite::Connection::open(f.data.path().join(REGISTRY_FILE)).unwrap();
    c.execute(
        "DELETE FROM owned_generated_keys WHERE key_id=?1",
        [f.registration.id.to_string()],
    )
    .unwrap();
    assert_eq!(
        f.service.inspect_selected_key(&f.store).unwrap_err().kind,
        KeyMaterialErrorKind::OwnershipUnverified
    );
    let (mut session, _) = credentials([]);
    assert_eq!(
        f.service
            .unlock_generated_key(&f.store, &mut session)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OwnershipUnverified
    );
}

#[cfg(unix)]
#[test]
fn generated_inspection_rejects_links_and_unprotected_private_source() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = SelectedFixture::new(false);
    fs::set_permissions(
        &f.registration.private_key_path,
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        f.service.inspect_selected_key(&f.store).unwrap_err().kind,
        KeyMaterialErrorKind::ProtectionUnavailable
    );
    fs::set_permissions(
        &f.registration.private_key_path,
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let moved = f._home.path().join("moved");
    fs::rename(&f.registration.private_key_path, &moved).unwrap();
    symlink(&moved, &f.registration.private_key_path).unwrap();
    assert_eq!(
        f.service.inspect_selected_key(&f.store).unwrap_err().kind,
        KeyMaterialErrorKind::UnsafePath
    );
}

use manyhands::repository::keys::DeleteGeneratedKeyOutcome;
use manyhands::repository::{RepositoryErrorKind, UnregisterSharedKeyOutcome};

fn deletion_fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    KeyStore,
    SharedKeyRegistration,
) {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let r = created(
        service
            .generate_shared_key(
                &store,
                request(OperationId::new(), KeyProtection::Unencrypted),
            )
            .unwrap(),
    );
    (home, data, service, store, r)
}

#[test]
fn deletion_removes_exact_owned_pair() {
    let (home, data, service, store, r) = deletion_fixture();
    let sentinel = home.path().join("sentinel");
    fs::write(&sentinel, b"untouched").unwrap();
    let other = created(
        service
            .generate_shared_key(
                &store,
                request(OperationId::new(), KeyProtection::Unencrypted),
            )
            .unwrap(),
    );
    let other_bytes = fs::read(&other.private_key_path).unwrap();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    assert_eq!(review.registration(), &r);
    let replay_review = service.review_generated_key_deletion(&store, r.id).unwrap();
    let id = OperationId::new();
    assert_eq!(
        service
            .delete_generated_key(&store, id, Some(review), true)
            .unwrap(),
        DeleteGeneratedKeyOutcome::Deleted
    );
    assert!(!r.private_key_path.exists() && !r.public_key_path.as_ref().unwrap().exists());
    assert!(
        fs::read(&other.private_key_path).unwrap() == other_bytes,
        "other key changed"
    );
    assert!(
        fs::read(sentinel).unwrap() == b"untouched",
        "sentinel changed"
    );
    fs::write(&r.private_key_path, b"replacement").unwrap();
    fs::write(r.public_key_path.as_ref().unwrap(), b"replacement-public").unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, id, Some(replay_review), true)
            .unwrap(),
        DeleteGeneratedKeyOutcome::AlreadyDeleted
    );
    drop(service);
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, id, None, true)
            .unwrap(),
        DeleteGeneratedKeyOutcome::AlreadyDeleted
    );
    assert!(
        fs::read(&r.private_key_path).unwrap() == b"replacement",
        "replacement changed"
    );
    assert!(
        fs::read(r.public_key_path.as_ref().unwrap()).unwrap() == b"replacement-public",
        "replacement changed"
    );
}

#[test]
fn deletion_cancel_writes_nothing() {
    let (_home, data, service, store, r) = deletion_fixture();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    let bytes = fs::read(&r.private_key_path).unwrap();
    let id = OperationId::new();
    assert_eq!(
        service
            .delete_generated_key(&store, id, Some(review), false)
            .unwrap(),
        DeleteGeneratedKeyOutcome::Cancelled
    );
    assert_eq!(
        service
            .delete_generated_key(&store, id, None, false)
            .unwrap(),
        DeleteGeneratedKeyOutcome::Cancelled
    );
    assert!(
        fs::read(&r.private_key_path).unwrap() == bytes,
        "private source changed"
    );
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM key_material_operations WHERE operation_id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(service.list_shared_keys().unwrap(), vec![r]);
}

#[test]
fn deletion_rechecks_selection_after_review() {
    let (_home, data, service, store, r) = deletion_fixture();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    let other = RepositoryService::open_at(data.path()).unwrap();
    other.select_shared_key(r.id).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, OperationId::new(), Some(review), true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::SelectedKeyMustBeCleared
    );
    assert!(r.private_key_path.exists() && r.public_key_path.unwrap().exists());
}

#[test]
fn deletion_refuses_imported_selected_and_unproven_generated_rows() {
    let (home, data, service, store, r) = deletion_fixture();
    service.select_shared_key(r.id).unwrap();
    assert_eq!(
        service
            .review_generated_key_deletion(&store, r.id)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::SelectedKeyMustBeCleared
    );
    service.clear_shared_key_selection().unwrap();
    for (ownership, expected_path) in [
        (SharedKeyOwnership::Imported, false),
        (SharedKeyOwnership::Generated, false),
        (SharedKeyOwnership::Generated, true),
    ] {
        let path = if expected_path {
            home.path().join(".ssh/manyhands/forged")
        } else {
            home.path().join(format!("sentinel-{}", OperationId::new()))
        };
        fs::write(&path, b"outside sentinel").unwrap();
        let RegisterSharedKeyOutcome::Registered(fake) = service
            .register_shared_key(RegisterSharedKeyRequest {
                label: "forged".into(),
                ownership,
                private_key_path: path.clone(),
                public_key_path: None,
            })
            .unwrap()
        else {
            panic!("registration")
        };
        if expected_path {
            let expected = home.path().join(".ssh/manyhands").join(fake.id.to_string());
            fs::rename(&path, &expected).unwrap();
            let c = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            c.execute("UPDATE shared_ssh_keys SET private_key_path=?2,public_key_path=?3,public_metadata_state='unavailable' WHERE id=?1", rusqlite::params![fake.id.to_string(),expected.to_str(),expected.with_extension("pub").to_str()]).unwrap();
        }
        assert_eq!(
            service
                .review_generated_key_deletion(&store, fake.id)
                .unwrap_err()
                .kind,
            if ownership == SharedKeyOwnership::Imported {
                KeyMaterialErrorKind::ImportedKey
            } else {
                KeyMaterialErrorKind::OwnershipUnverified
            }
        );
        let path = if expected_path {
            home.path().join(".ssh/manyhands").join(fake.id.to_string())
        } else {
            path
        };
        assert!(
            fs::read(path).unwrap() == b"outside sentinel",
            "unowned sentinel changed"
        );
    }
}

#[test]
fn deletion_refuses_replaced_or_linked_target() {
    for variant in 0..4 {
        let (home, _data, service, store, r) = deletion_fixture();
        let review = service.review_generated_key_deletion(&store, r.id).unwrap();
        let path = if variant == 3 {
            r.public_key_path.as_ref().unwrap()
        } else {
            &r.private_key_path
        };
        let sentinel = home.path().join("sentinel");
        fs::rename(path, &sentinel).unwrap();
        let bytes = fs::read(&sentinel).unwrap();
        match variant {
            0 | 3 => {
                fs::copy(&sentinel, path).unwrap();
            }
            1 => {
                fs::hard_link(&sentinel, path).unwrap();
            }
            _ => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&sentinel, path).unwrap();
                #[cfg(windows)]
                fs::copy(&sentinel, path).unwrap();
            }
        }
        assert!(
            service
                .delete_generated_key(&store, OperationId::new(), Some(review), true)
                .is_err()
        );
        assert!(service.review_generated_key_deletion(&store, r.id).is_err());
        assert!(
            fs::read(&sentinel).unwrap() == bytes,
            "outside source changed"
        );
        assert!(path.exists());
    }
}

#[test]
fn deletion_retry_requires_fresh_confirmation() {
    for (point, phase, private_exists, public_exists) in [
        (
            FailurePoint::DeletionAfterIntent,
            KeyMaterialPhase::Prepared,
            true,
            true,
        ),
        (
            FailurePoint::DeletionAfterPrivateUnlink,
            KeyMaterialPhase::Prepared,
            false,
            true,
        ),
        (
            FailurePoint::DeletionAfterPrivatePhase,
            KeyMaterialPhase::PrivateRemoved,
            false,
            true,
        ),
        (
            FailurePoint::DeletionAfterPublicUnlink,
            KeyMaterialPhase::PrivateRemoved,
            false,
            false,
        ),
        (
            FailurePoint::DeletionAfterFilesPhase,
            KeyMaterialPhase::FilesRemoved,
            false,
            false,
        ),
        (
            FailurePoint::DeletionBeforeFinalTransaction,
            KeyMaterialPhase::FilesRemoved,
            false,
            false,
        ),
        (
            FailurePoint::DeletionAfterFinalTransaction,
            KeyMaterialPhase::Completed,
            false,
            false,
        ),
    ] {
        let (_home, data, service, store, r) = deletion_fixture();
        drop(service);
        let service =
            RepositoryService::open_at_with_failure_point_for_testing(data.path(), point).unwrap();
        let review = service.review_generated_key_deletion(&store, r.id).unwrap();
        let id = OperationId::new();
        let DeleteGeneratedKeyOutcome::RecoveryRequired(recovery) = service
            .delete_generated_key(&store, id, Some(review), true)
            .unwrap()
        else {
            panic!("expected interruption")
        };
        assert_eq!(recovery.phase, phase);
        assert_eq!(recovery.action, KeyMaterialAction::Delete);
        drop(service);
        let service = RepositoryService::open_at(data.path()).unwrap();
        assert_eq!(r.private_key_path.exists(), private_exists);
        assert_eq!(r.public_key_path.as_ref().unwrap().exists(), public_exists);
        if phase == KeyMaterialPhase::Completed {
            assert_eq!(
                service
                    .delete_generated_key(&store, id, None, true)
                    .unwrap(),
                DeleteGeneratedKeyOutcome::AlreadyDeleted
            );
        } else {
            assert_eq!(service.list_shared_keys().unwrap().len(), 1);
            assert_eq!(
                service.select_shared_key(r.id).unwrap_err().kind,
                RepositoryErrorKind::SharedKeyMaterialPending
            );
            assert_eq!(
                service.unregister_shared_key(r.id).unwrap_err().kind,
                RepositoryErrorKind::SharedKeyMaterialPending
            );
            assert_eq!(
                service
                    .delete_generated_key(&store, id, None, true)
                    .unwrap_err()
                    .kind,
                KeyMaterialErrorKind::ConfirmationRequired
            );
            let review = service.review_generated_key_deletion(&store, r.id).unwrap();
            assert_eq!(
                service
                    .delete_generated_key(&store, OperationId::new(), Some(review), true)
                    .unwrap_err()
                    .kind,
                KeyMaterialErrorKind::Busy
            );
            let review = service.review_generated_key_deletion(&store, r.id).unwrap();
            assert_eq!(
                service
                    .delete_generated_key(&store, id, Some(review), true)
                    .unwrap(),
                DeleteGeneratedKeyOutcome::Deleted
            );
        }
        assert!(service.list_shared_keys().unwrap().is_empty());
        assert!(!r.private_key_path.exists() && !r.public_key_path.unwrap().exists());
    }
}

#[test]
fn deletion_retry_preserves_replacement_files() {
    let (_home, data, service, store, r) = deletion_fixture();
    drop(service);
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        data.path(),
        FailurePoint::DeletionAfterPrivateUnlink,
    )
    .unwrap();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    let id = OperationId::new();
    service
        .delete_generated_key(&store, id, Some(review), true)
        .unwrap();
    fs::write(&r.private_key_path, b"replacement").unwrap();
    assert!(service.review_generated_key_deletion(&store, r.id).is_err());
    assert_eq!(
        service
            .delete_generated_key(&store, id, None, true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::ConfirmationRequired
    );
    assert!(
        fs::read(&r.private_key_path).unwrap() == b"replacement",
        "replacement changed"
    );
    assert!(r.public_key_path.unwrap().exists());
}

#[test]
fn deletion_operation_id_mismatch_changes_nothing() {
    let (_home, _data, service, store, r) = deletion_fixture();
    let generation_id = OperationId::new();
    let other = created(
        service
            .generate_shared_key(&store, request(generation_id, KeyProtection::Unencrypted))
            .unwrap(),
    );
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, generation_id, Some(review), true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OperationMismatch
    );
    let id = OperationId::new();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    service
        .delete_generated_key(&store, id, Some(review), true)
        .unwrap();
    let review = service
        .review_generated_key_deletion(&store, other.id)
        .unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, id, Some(review), true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OperationMismatch
    );
    let other_home = tempfile::tempdir().unwrap();
    let other_store = KeyStore::for_home(other_home.path()).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&other_store, id, None, true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OperationMismatch
    );
    assert!(other.private_key_path.exists() && other.public_key_path.unwrap().exists());
    assert!(!other_home.path().join(".ssh").exists());
}

#[test]
fn deletion_missing_entries_require_fresh_review() {
    let (_home, _data, service, store, r) = deletion_fixture();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    fs::remove_file(&r.private_key_path).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, OperationId::new(), Some(review), true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::SourceChanged
    );
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, OperationId::new(), Some(review), true)
            .unwrap(),
        DeleteGeneratedKeyOutcome::Deleted
    );
}

#[test]
fn unregister_generated_retains_both_files() {
    let (_home, _data, service, store, r) = deletion_fixture();
    let private = fs::read(&r.private_key_path).unwrap();
    let public = fs::read(r.public_key_path.as_ref().unwrap()).unwrap();
    assert_eq!(
        service.unregister_shared_key(r.id).unwrap(),
        UnregisterSharedKeyOutcome::Unregistered
    );
    assert!(
        fs::read(&r.private_key_path).unwrap() == private,
        "private source changed"
    );
    assert!(
        fs::read(r.public_key_path.unwrap()).unwrap() == public,
        "public source changed"
    );
    assert_eq!(
        service
            .review_generated_key_deletion(&store, r.id)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::NotRegistered
    );
}

#[test]
fn deletion_cancel_without_review_does_not_create_store() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, OperationId::new(), None, false)
            .unwrap(),
        DeleteGeneratedKeyOutcome::Cancelled
    );
    assert!(!home.path().join(".ssh").exists());
    assert!(service.list_key_material_recovery().unwrap().is_empty());
}

#[test]
fn deletion_refuses_in_place_changes_and_changed_creation_evidence() {
    for change_evidence in [false, true] {
        let (_home, data, service, store, r) = deletion_fixture();
        let review = service.review_generated_key_deletion(&store, r.id).unwrap();
        if change_evidence {
            let c = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            c.execute("UPDATE owned_generated_keys SET public_file_identity=private_file_identity WHERE key_id=?1", [r.id.to_string()]).unwrap();
        } else {
            fs::write(&r.private_key_path, b"changed in place").unwrap();
        }
        assert!(
            service
                .delete_generated_key(&store, OperationId::new(), Some(review), true)
                .is_err()
        );
        assert!(r.private_key_path.exists() && r.public_key_path.unwrap().exists());
    }
}

#[test]
fn deletion_missing_pair_still_requires_creation_evidence() {
    let (_home, data, service, store, r) = deletion_fixture();
    fs::remove_file(&r.private_key_path).unwrap();
    fs::remove_file(r.public_key_path.as_ref().unwrap()).unwrap();
    let review = service.review_generated_key_deletion(&store, r.id).unwrap();
    let c = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    c.execute(
        "DELETE FROM owned_generated_keys WHERE key_id=?1",
        [r.id.to_string()],
    )
    .unwrap();
    assert_eq!(
        service
            .delete_generated_key(&store, OperationId::new(), Some(review), true)
            .unwrap_err()
            .kind,
        KeyMaterialErrorKind::OwnershipUnverified
    );
    assert_eq!(service.list_shared_keys().unwrap().len(), 1);
}
