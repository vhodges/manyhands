//! Regression: persisting or formatting any credential payload must fail this test.
use super::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const CHILD: &str = "MANYHANDS_CREDENTIAL_PRIVACY_ROOT";
const PASSWORD: &str = "MANYHANDS_CREDENTIAL_PRIVACY_PASSWORD";

fn assert_clean(bytes: &[u8], probes: &[Vec<u8>]) {
    assert!(
        probes.iter().all(|probe| !probe.is_empty()),
        "privacy probes must be nonempty"
    );
    assert!(
        probes
            .iter()
            .all(|probe| !bytes.windows(probe.len()).any(|w| w == probe)),
        "credential material escaped its approved file"
    );
}

fn scan_tree(root: &Path, probes: &[Vec<u8>]) -> Vec<(PathBuf, usize)> {
    let mut scanned = Vec::new();
    for entry in fs::read_dir(root).expect("read isolated application data") {
        let path = entry.expect("read application entry").path();
        let metadata = fs::symlink_metadata(&path).expect("inspect application entry");
        assert!(
            !metadata.file_type().is_symlink(),
            "unexpected application data link"
        );
        if metadata.is_dir() {
            scanned.extend(scan_tree(&path, probes));
        } else {
            let bytes = fs::read(&path).expect("read application file");
            assert_clean(&bytes, probes);
            assert_clean(path.to_string_lossy().as_bytes(), probes);
            scanned.push((path, bytes.len()));
        }
    }
    scanned
}

fn probes(home: &Path, password: &str) -> Vec<Vec<u8>> {
    let mut result = vec![
        password.as_bytes().to_vec(),
        format!("wrong-{password}").into_bytes(),
        format!("malformed-{password}").into_bytes(),
    ];
    // These are deliberate private-key fixtures under the isolated SSH home,
    // never application data. Keep them to probe even after deletion/mutation.
    for entry in fs::read_dir(home.join("fixtures")).expect("read private fixtures") {
        let encoded =
            fs::read(entry.expect("read fixture entry").path()).expect("read private fixture");
        let key = ssh_key::PrivateKey::from_openssh(&encoded).expect("parse generated fixture");
        let key = if key.is_encrypted() {
            key.decrypt(password).expect("decrypt generated fixture")
        } else {
            key
        };
        let seed = key
            .key_data()
            .ed25519()
            .expect("generated Ed25519")
            .private
            .to_bytes();
        result.push(seed[..16].to_vec());
        // Check individual PEM body lines as well as the whole encoding.
        result.extend(
            encoded
                .split(|b| *b == b'\n')
                .filter(|line| !line.is_empty() && !line.starts_with(b"-----"))
                .map(<[u8]>::to_vec),
        );
        result.push(encoded);
    }
    assert!(
        result.len() > 5,
        "private material probes must be populated"
    );
    result
}

#[test]
fn credential_outputs_and_storage_exclude_secrets() {
    if let Some(root) = std::env::var_os(CHILD) {
        exercise(
            &PathBuf::from(root),
            &std::env::var(PASSWORD).expect("isolated password"),
        );
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let password = format!("privacy-{}", OperationId::new());
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "privacy::credential_outputs_and_storage_exclude_secrets",
            "--nocapture",
        ])
        .env(CHILD, root.path())
        .env(PASSWORD, &password)
        .output()
        .expect("run isolated privacy subprocess");
    // Never forward captured output, including on failure: it may contain a leak.
    assert!(
        output.status.success(),
        "privacy subprocess failed; captured output withheld"
    );
    let probes = probes(&root.path().join("ssh-home"), &password);
    assert_clean(&output.stdout, &probes);
    assert_clean(&output.stderr, &probes);
    assert!(
        output
            .stdout
            .windows(24)
            .any(|w| w == b"privacy scenarios passed"),
        "privacy subprocess must execute its scenarios"
    );
    assert!(
        !scan_tree(&root.path().join("app-data"), &probes).is_empty(),
        "application files must be scanned"
    );
}

fn exercise(root: &Path, password: &str) {
    let home = root.join("ssh-home");
    let data = root.join("app-data/nested/registry");
    fs::create_dir_all(home.join("fixtures")).unwrap();
    fs::create_dir_all(&data).unwrap();
    let service = RepositoryService::open_at(&data).unwrap();
    let store = KeyStore::for_home(&home).unwrap();
    let database = data.join(REGISTRY_FILE);
    let reader = rusqlite::Connection::open(&database).unwrap();
    reader
        .execute_batch("BEGIN; SELECT count(*) FROM shared_ssh_keys;")
        .unwrap();
    let protection =
        || KeyProtection::Passphrase(SecretPassphrase::new(password.to_owned()).unwrap());
    let generated = service
        .generate_shared_key(&store, request(OperationId::new(), protection()))
        .unwrap();
    println!("generation: {generated:?}");
    let registration = created(generated);
    fs::copy(
        &registration.private_key_path,
        home.join("fixtures/generated"),
    )
    .unwrap();
    service.select_shared_key(registration.id).unwrap();
    let (mut session, _) = credentials([
        supplied(&format!("wrong-{password}")),
        PassphraseResponse::Cancelled,
        supplied(password),
    ]);
    let error = service
        .unlock_generated_key(&store, &mut session)
        .unwrap_err();
    assert!(
        error.kind == KeyMaterialErrorKind::UnlockFailed,
        "wrong password must reject"
    );
    eprintln!("wrong password: {error}; {error:?}; {}", error.guidance());
    let cancelled = service.unlock_generated_key(&store, &mut session).unwrap();
    assert!(
        matches!(cancelled, GeneratedKeyUnlockOutcome::Cancelled),
        "cancel must propagate"
    );
    println!("cancel: {cancelled:?}; session: {session:?}");
    let unlocked = service.unlock_generated_key(&store, &mut session).unwrap();
    assert!(
        matches!(unlocked, GeneratedKeyUnlockOutcome::Ready(_)),
        "correct password must unlock"
    );
    println!("unlock: {unlocked:?}; session: {session:?}");
    session.clear();
    fs::write(
        &registration.private_key_path,
        format!("malformed-{password}"),
    )
    .unwrap();
    let error = service
        .unlock_generated_key(&store, &mut session)
        .unwrap_err();
    assert!(
        error.kind == KeyMaterialErrorKind::InvalidGeneratedKey,
        "malformed key must reject"
    );
    eprintln!("malformed: {error}; {error:?}; {}", error.guidance());
    denied_io(&home, &service, &store, password);
    service.clear_shared_key_selection().unwrap();
    let writer = rusqlite::Connection::open(&database).unwrap();
    writer.execute_batch("CREATE TRIGGER reject_registration BEFORE INSERT ON shared_ssh_keys BEGIN SELECT RAISE(ABORT, 'registration denied'); END;").unwrap();
    let failure = service
        .generate_shared_key(&store, request(OperationId::new(), protection()))
        .unwrap();
    assert!(
        matches!(failure, GenerateSharedKeyOutcome::RecoveryRequired(ref r) if r.phase == KeyMaterialPhase::PairWritten),
        "registration failure must retain pair"
    );
    if let GenerateSharedKeyOutcome::RecoveryRequired(ref recovery) = failure {
        fs::copy(
            home.join(".ssh/manyhands")
                .join(recovery.key_id.to_string()),
            home.join("fixtures/unregistered"),
        )
        .unwrap();
    }
    println!("registration failure: {failure:?}");
    writer
        .execute_batch("DROP TRIGGER reject_registration")
        .unwrap();
    let deletion_key = created(
        service
            .generate_shared_key(
                &store,
                request(OperationId::new(), KeyProtection::Unencrypted),
            )
            .unwrap(),
    );
    fs::copy(
        &deletion_key.private_key_path,
        home.join("fixtures/deleted"),
    )
    .unwrap();
    let interrupted = RepositoryService::open_at_with_failure_point_for_testing(
        &data,
        FailurePoint::DeletionAfterPrivateUnlink,
    )
    .unwrap();
    let review = interrupted
        .review_generated_key_deletion(&store, deletion_key.id)
        .unwrap();
    let deletion = interrupted
        .delete_generated_key(&store, OperationId::new(), Some(review), true)
        .unwrap();
    assert!(
        matches!(deletion, DeleteGeneratedKeyOutcome::RecoveryRequired(_)),
        "deletion must interrupt"
    );
    println!(
        "deletion: {deletion:?}; recovery: {:?}",
        interrupted.list_key_material_recovery().unwrap()
    );
    let probes = probes(&home, password);
    let wal = data.join(format!("{REGISTRY_FILE}-wal"));
    assert!(
        fs::metadata(&wal).unwrap().len() > 32,
        "active WAL must contain frames"
    );
    let live_scan = scan_tree(&root.join("app-data"), &probes);
    assert!(
        live_scan
            .iter()
            .any(|(path, bytes)| path == &wal && *bytes > 32),
        "live WAL frames must be scanned with reader active"
    );
    // Snapshot actual DB/WAL bytes into nested recovery-backup paths while the
    // reader pins WAL frames; scan all artifacts without extension filtering.
    let backup = root.join("app-data/recovery/backups");
    fs::create_dir_all(&backup).unwrap();
    fs::copy(&database, backup.join("registry.sqlite3.corrupt-snapshot")).unwrap();
    fs::copy(&wal, backup.join("registry.sqlite3-wal.backup")).unwrap();
    reader.execute_batch("ROLLBACK").unwrap();
    writer
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .unwrap();
    fs::copy(&database, backup.join("rollback.sqlite3")).unwrap();
    let rollback = rusqlite::Connection::open(backup.join("rollback.sqlite3")).unwrap();
    rollback.execute_batch("PRAGMA journal_mode=DELETE; BEGIN IMMEDIATE; UPDATE shared_ssh_keys SET label='privacy rollback';").unwrap();
    let journal = backup.join("rollback.sqlite3-journal");
    assert!(
        fs::metadata(&journal).unwrap().len() > 0,
        "live rollback journal must exist"
    );
    let scanned = scan_tree(&root.join("app-data"), &probes);
    for required in [
        &database,
        &wal,
        &journal,
        &backup.join("registry.sqlite3.corrupt-snapshot"),
        &backup.join("registry.sqlite3-wal.backup"),
    ] {
        assert!(
            scanned.iter().any(|(path, _)| path == required),
            "required artifact was not scanned"
        );
    }
    assert!(
        scanned.iter().map(|(_, bytes)| bytes).sum::<usize>() > 8192,
        "scan must cover populated storage"
    );
    // Scan the pinned live WAL before checkpoint as well as the retained copy.
    assert_clean(
        &fs::read(backup.join("registry.sqlite3-wal.backup")).unwrap(),
        &probes,
    );
    rollback.execute_batch("ROLLBACK").unwrap();
    println!("privacy scenarios passed");
}

fn denied_io(home: &Path, service: &RepositoryService, store: &KeyStore, password: &str) {
    let path = home.join("denied-source");
    fs::write(&path, password).unwrap();
    import_selected(service, &path);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(
            unsafe { libc::geteuid() } != 0,
            "denied-read privacy test requires an unprivileged runner"
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    }
    #[cfg(windows)]
    let _exclusive = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap()
    };
    let error = service.inspect_selected_key(store).unwrap_err();
    assert!(
        error.kind == KeyMaterialErrorKind::SourceUnreadable,
        "denied source must be unreadable"
    );
    eprintln!("denied IO: {error}; {error:?}; {}", error.guidance());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}
