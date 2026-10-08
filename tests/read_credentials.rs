//! The credential reads: key registrations, public key text and host pins.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use manyhands::{
    repository::{
        HostPinDto, KeyDto, KeyOwnership, KeyPrivateSourceState, KeyPublicMetadataState, LeaseKind,
        ReadError, RepositoryService, SharedKeyId, transport::SshAuthority,
    },
    results::{Envelope, FailureClass, Outcome, ResultCode, Scope},
};
use serde::Serialize;
use serde_json::Value;
use support::{
    credentials::{
        self, OTHER_FINGERPRINT, PUBLIC_FIXTURE_FINGERPRINT, RegisteredKeys, SECRET_SENTINELS,
    },
    golden,
};

mod support;

/// Far above the 250 ms lease bound and far below a blocked wait.
const NOT_BLOCKED: Duration = Duration::from_secs(5);

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

#[test]
fn credentials_lease_child() {
    let Ok(root) = std::env::var("MANYHANDS_LEASE_ROOT") else {
        return;
    };
    let data_directory = PathBuf::from(std::env::var("MANYHANDS_LEASE_DATA_DIRECTORY").unwrap());
    let kind = LeaseKind::parse(&std::env::var("MANYHANDS_LEASE_KIND").unwrap()).unwrap();
    let ready = PathBuf::from(std::env::var("MANYHANDS_LEASE_READY").unwrap());
    let release = PathBuf::from(std::env::var("MANYHANDS_LEASE_RELEASE").unwrap());
    let _holder =
        RepositoryService::hold_lease_for_testing(Path::new(&root), &data_directory, kind).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !release.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct Fixture {
    service: RepositoryService,
    data: tempfile::TempDir,
    keys: RegisteredKeys,
}

fn fixture() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let keys = credentials::register_keys(&service);
    Fixture {
        service,
        data,
        keys,
    }
}

impl Fixture {
    fn hold_index_lock(&self, kind: LeaseKind) -> support::LeaseHolder {
        support::hold_lease_in_child_for_test(
            self.data.path(),
            self.data.path(),
            kind,
            "credentials_lease_child",
        )
    }

    fn pin(&self, host: &str, port: u16, algorithm: &str, sha256: &str) {
        credentials::pin_host(self.data.path(), host, port, algorithm, sha256);
    }
}

fn authority(host: &str, port: u16) -> SshAuthority {
    SshAuthority {
        host: host.to_owned(),
        port,
    }
}

fn path_text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

/// The envelope a front end would print, searched for every secret.
fn assert_no_secret(command: &str, data: &impl Serialize) {
    let envelope = Envelope::read_success(command, Scope::default(), data);
    let value = serde_json::to_value(&envelope).unwrap();
    assert_eq!(golden::find_sentinel(&value, &SECRET_SENTINELS), None);
}

fn assert_failure_has_no_secret(command: &str, error: &ReadError) {
    let value = serde_json::to_value(error.to_envelope::<Value>(command)).unwrap();
    assert_eq!(golden::find_sentinel(&value, &SECRET_SENTINELS), None);
    assert_eq!(value["data"], Value::Null);
}

#[test]
fn listing_returns_each_registration_as_it_is_stored_in_registration_order() {
    let fixture = fixture();
    let keys = &fixture.keys;

    let list = fixture.service.list_keys().unwrap();

    assert!(list.complete);
    assert_eq!(
        list.items,
        [
            KeyDto {
                id: keys.imported.id.to_string(),
                label: "Imported key".to_owned(),
                ownership: KeyOwnership::Imported,
                selected: false,
                fingerprint: Some(PUBLIC_FIXTURE_FINGERPRINT.to_owned()),
                private_source_state: KeyPrivateSourceState::Available,
                public_metadata_state: KeyPublicMetadataState::Available,
                private_key_path: path_text(&keys.imported.private_key_path),
                public_key_path: keys.imported.public_key_path.as_deref().map(path_text),
            },
            KeyDto {
                id: keys.generated.id.to_string(),
                label: "Generated key".to_owned(),
                ownership: KeyOwnership::Generated,
                selected: true,
                fingerprint: keys.generated.public_key_fingerprint.clone(),
                private_source_state: KeyPrivateSourceState::Available,
                public_metadata_state: KeyPublicMetadataState::Available,
                private_key_path: path_text(&keys.generated.private_key_path),
                public_key_path: keys.generated.public_key_path.as_deref().map(path_text),
            },
            KeyDto {
                id: keys.without_public.id.to_string(),
                label: "Key without a public file".to_owned(),
                ownership: KeyOwnership::Imported,
                selected: false,
                fingerprint: None,
                private_source_state: KeyPrivateSourceState::Available,
                public_metadata_state: KeyPublicMetadataState::NotProvided,
                private_key_path: path_text(&keys.without_public.private_key_path),
                public_key_path: None,
            },
        ]
    );
    assert!(keys.generated.public_key_fingerprint.is_some());
    // The order is that of registration, which the existing listing uses.
    let existing: Vec<_> = fixture
        .service
        .list_shared_keys()
        .unwrap()
        .into_iter()
        .map(|registration| registration.id.to_string())
        .collect();
    let listed: Vec<_> = list.items.iter().map(|key| key.id.clone()).collect();
    assert_eq!(listed, existing);
    for key in &list.items {
        let id = SharedKeyId::parse(&key.id).unwrap();
        assert_eq!(&fixture.service.show_key(id).unwrap(), key);
    }
    assert_no_secret("key list", &list);
    assert_git_transport_uninitialized();
}

#[test]
fn a_stored_state_is_reported_without_observing_the_file_again() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let files = tempfile::tempdir().unwrap();
    let directory = fs::canonicalize(files.path()).unwrap();
    let absent = directory.join("absent");
    let not_a_key = directory.join("not-a-key.pub");
    fs::write(&not_a_key, "not a public key\n").unwrap();
    let registered = service
        .register_shared_key(manyhands::repository::RegisterSharedKeyRequest {
            label: "Missing".to_owned(),
            ownership: manyhands::repository::SharedKeyOwnership::Imported,
            private_key_path: absent.clone(),
            public_key_path: Some(not_a_key.clone()),
        })
        .unwrap();
    let manyhands::repository::RegisterSharedKeyOutcome::Registered(registered) = registered else {
        panic!("not registered");
    };
    // Both files are now what a fresh observation would call available.
    fs::write(&absent, credentials::private_fixture_bytes()).unwrap();
    fs::write(&not_a_key, credentials::public_fixture_text()).unwrap();

    let key = service.show_key(registered.id).unwrap();

    assert_eq!(key.private_source_state, KeyPrivateSourceState::Missing);
    assert_eq!(
        key.public_metadata_state,
        KeyPublicMetadataState::Unavailable
    );
    assert_eq!(key.fingerprint, None);
    assert_eq!(key.public_key_path, Some(path_text(&not_a_key)));
    assert_eq!(service.list_keys().unwrap().items, [key]);
    assert_git_transport_uninitialized();
}

#[test]
fn an_unknown_key_is_not_found() {
    let fixture = fixture();
    let unknown = SharedKeyId::new();

    let errors = [
        fixture.service.show_key(unknown).unwrap_err(),
        fixture.service.public_key_text(unknown).unwrap_err(),
    ];

    for error in &errors {
        assert_eq!(error.code(), ResultCode::KeyNotFound);
        assert_eq!(error.code().failure_class(), Some(FailureClass::Input));
        assert_eq!(error.scope, Scope::default());
        assert!(error.recovery.is_empty());
    }
    let data = tempfile::tempdir().unwrap();
    let empty = RepositoryService::open_at(data.path()).unwrap();
    assert!(empty.list_keys().unwrap().items.is_empty());
    assert!(empty.list_keys().unwrap().complete);
    assert_eq!(
        empty.show_key(unknown).unwrap_err().code(),
        ResultCode::KeyNotFound
    );
    assert_git_transport_uninitialized();
}

#[test]
fn key_reads_succeed_under_the_shared_lock_and_are_busy_under_the_exclusive_lock() {
    let fixture = fixture();
    let id = fixture.keys.imported.id;
    let expected = fixture.service.list_keys().unwrap();

    let shared = fixture.hold_index_lock(LeaseKind::CacheRead);
    let started = Instant::now();
    assert_eq!(fixture.service.list_keys().unwrap(), expected);
    assert_eq!(fixture.service.show_key(id).unwrap(), expected.items[0]);
    fixture.service.public_key_text(id).unwrap();
    assert!(started.elapsed() < NOT_BLOCKED);
    shared.release();

    let exclusive = fixture.hold_index_lock(LeaseKind::CacheWrite);
    let started = Instant::now();
    let busy = [
        fixture.service.list_keys().unwrap_err(),
        fixture.service.show_key(id).unwrap_err(),
        fixture.service.public_key_text(id).unwrap_err(),
        fixture.service.show_key(SharedKeyId::new()).unwrap_err(),
    ];
    assert!(started.elapsed() < NOT_BLOCKED);
    for error in &busy {
        assert_eq!(error.code(), ResultCode::Busy);
        assert!(error.recovery.is_empty());
        assert_eq!(
            error.to_envelope::<Value>("key list").outcome,
            Outcome::Error
        );
    }

    // The lock, not the registry, was the obstacle.
    exclusive.release();
    assert_eq!(fixture.service.list_keys().unwrap(), expected);
    assert_git_transport_uninitialized();
}

/// Every key read, with the envelope of each searched for a secret.
fn every_key_read(fixture: &Fixture) -> Vec<Value> {
    let keys = &fixture.keys;
    let list = fixture.service.list_keys().unwrap();
    assert_no_secret("key list", &list);
    let mut results = vec![serde_json::to_value(&list).unwrap()];
    for registration in [&keys.imported, &keys.generated, &keys.without_public] {
        let key = fixture.service.show_key(registration.id).unwrap();
        assert_no_secret("key show", &key);
        results.push(serde_json::to_value(&key).unwrap());
        match fixture.service.public_key_text(registration.id) {
            Ok(public) => {
                assert_no_secret("key public", &public);
                results.push(serde_json::to_value(&public).unwrap());
            }
            Err(error) => {
                assert_eq!(error.code(), ResultCode::PublicKeyUnavailable);
                assert_eq!(registration.id, keys.without_public.id);
                assert_failure_has_no_secret("key public", &error);
                results.push(Value::Null);
            }
        }
    }
    results
}

#[test]
fn key_reads_leave_every_key_file_as_it_was() {
    let fixture = fixture();
    let files = fixture.keys.key_files();
    assert_eq!(files.len(), 5);
    let before = credentials::key_file_states(&files);
    assert!(before.iter().all(|state| state.entry.is_some()));
    // The secrets really are in the files, so the scan can fail.
    let imported = fs::read(&fixture.keys.imported.private_key_path).unwrap();
    assert!(String::from_utf8_lossy(&imported).contains(credentials::PRIVATE_KEY_SENTINEL));
    let generated = fs::read(&fixture.keys.generated.private_key_path).unwrap();
    assert!(String::from_utf8_lossy(&generated).contains("PRIVATE KEY"));

    let results = every_key_read(&fixture);

    assert_eq!(results.len(), 7);
    assert_eq!(credentials::key_file_states(&files), before);
    assert_git_transport_uninitialized();
}

/// Whether this process can be denied a file by its permissions, which the
/// superuser cannot.
#[cfg(unix)]
fn permissions_bind(directory: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let probe = directory.join("permission-probe");
    fs::write(&probe, b"probe").unwrap();
    fs::set_permissions(&probe, fs::Permissions::from_mode(0o000)).unwrap();
    let bound = fs::read(&probe).is_err();
    fs::remove_file(&probe).unwrap();
    bound
}

#[cfg(unix)]
#[test]
fn no_key_read_opens_a_private_key_file() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = fixture();
    let keys = &fixture.keys;
    let expected = every_key_read(&fixture);
    assert!(permissions_bind(fixture.keys.files.path()));

    // Unreadable: any attempt to open one of these fails.
    for path in keys.private_key_files() {
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        assert!(fs::read(&path).is_err());
    }
    let unreadable = credentials::key_file_states(&keys.private_key_files());
    assert_eq!(every_key_read(&fixture), expected);
    assert_eq!(
        credentials::key_file_states(&keys.private_key_files()),
        unreadable
    );

    // A FIFO with no writer: opening one to read would block for ever.
    for path in [
        &keys.imported.private_key_path,
        &keys.without_public.private_key_path,
    ] {
        fs::remove_file(path).unwrap();
        let name = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    }
    let started = Instant::now();
    assert_eq!(every_key_read(&fixture), expected);
    assert!(started.elapsed() < NOT_BLOCKED);

    // A directory, and then nothing at all.
    fs::remove_file(&keys.imported.private_key_path).unwrap();
    fs::create_dir(&keys.imported.private_key_path).unwrap();
    assert_eq!(every_key_read(&fixture), expected);
    fs::remove_dir(&keys.imported.private_key_path).unwrap();
    fs::remove_file(&keys.without_public.private_key_path).unwrap();
    assert_eq!(every_key_read(&fixture), expected);

    // The stored state is still the one recorded at registration.
    for key in fixture.service.list_keys().unwrap().items {
        assert_eq!(key.private_source_state, KeyPrivateSourceState::Available);
    }
    assert_git_transport_uninitialized();
}

#[test]
fn public_key_text_is_the_file_text_and_a_fingerprint_computed_from_it() {
    let fixture = fixture();
    let keys = &fixture.keys;

    let imported = fixture.service.public_key_text(keys.imported.id).unwrap();
    let generated = fixture.service.public_key_text(keys.generated.id).unwrap();

    let text = credentials::public_fixture_text();
    assert!(text.ends_with('\n'));
    assert_eq!(imported.id, keys.imported.id.to_string());
    assert_eq!(imported.public_key, text.trim_end());
    assert_eq!(imported.fingerprint, PUBLIC_FIXTURE_FINGERPRINT);
    assert!(imported.matches_registration);
    let generated_text =
        fs::read_to_string(keys.generated.public_key_path.as_ref().unwrap()).unwrap();
    assert_eq!(generated.public_key, generated_text.trim_end());
    assert_eq!(
        Some(&generated.fingerprint),
        keys.generated.public_key_fingerprint.as_ref()
    );
    assert_eq!(
        generated.fingerprint,
        ssh_key::PublicKey::from_openssh(&generated_text)
            .unwrap()
            .fingerprint(ssh_key::HashAlg::Sha256)
            .to_string()
    );
    assert!(generated.matches_registration);
    assert_no_secret("key public", &imported);
    assert_no_secret("key public", &generated);
    assert_git_transport_uninitialized();
}

#[test]
fn a_replaced_public_key_file_does_not_match_its_registration() {
    let fixture = fixture();
    let keys = &fixture.keys;
    let imported_public = keys.imported.public_key_path.as_ref().unwrap();
    let generated_text =
        fs::read_to_string(keys.generated.public_key_path.as_ref().unwrap()).unwrap();
    // A different key, with CRLF line ending and no comment.
    fs::write(
        imported_public,
        format!("{}\r\n", generated_text.trim_end()),
    )
    .unwrap();

    let replaced = fixture.service.public_key_text(keys.imported.id).unwrap();

    assert_eq!(replaced.public_key, generated_text.trim_end());
    assert_eq!(
        Some(&replaced.fingerprint),
        keys.generated.public_key_fingerprint.as_ref()
    );
    assert_ne!(replaced.fingerprint, PUBLIC_FIXTURE_FINGERPRINT);
    assert!(!replaced.matches_registration);
    // The registration is not corrected by the read.
    let stored = fixture.service.show_key(keys.imported.id).unwrap();
    assert_eq!(
        stored.fingerprint.as_deref(),
        Some(PUBLIC_FIXTURE_FINGERPRINT)
    );

    // The same key with another comment is still the registered key.
    let (key, _comment) = credentials::public_fixture_text()
        .trim_end()
        .rsplit_once(' ')
        .map(|(key, comment)| (key.to_owned(), comment.to_owned()))
        .unwrap();
    fs::write(imported_public, format!("{key} another comment\n")).unwrap();
    let recommented = fixture.service.public_key_text(keys.imported.id).unwrap();
    assert_eq!(recommented.public_key, format!("{key} another comment"));
    assert!(recommented.matches_registration);
    assert_git_transport_uninitialized();
}

#[test]
fn a_registration_that_stores_no_fingerprint_matches_nothing() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let files = tempfile::tempdir().unwrap();
    let directory = fs::canonicalize(files.path()).unwrap();
    let public = directory.join("late.pub");
    fs::write(&public, "not yet a key\n").unwrap();
    let outcome = service
        .register_shared_key(manyhands::repository::RegisterSharedKeyRequest {
            label: "Late".to_owned(),
            ownership: manyhands::repository::SharedKeyOwnership::Imported,
            private_key_path: directory.join("late"),
            public_key_path: Some(public.clone()),
        })
        .unwrap();
    let manyhands::repository::RegisterSharedKeyOutcome::Registered(registered) = outcome else {
        panic!("not registered");
    };
    assert_eq!(registered.public_key_fingerprint, None);
    assert_eq!(
        service.public_key_text(registered.id).unwrap_err().code(),
        ResultCode::PublicKeyUnavailable
    );
    fs::write(&public, credentials::public_fixture_text()).unwrap();

    let public_key = service.public_key_text(registered.id).unwrap();

    assert_eq!(public_key.fingerprint, PUBLIC_FIXTURE_FINGERPRINT);
    assert!(!public_key.matches_registration);
    assert_git_transport_uninitialized();
}

fn assert_public_key_unavailable(fixture: &Fixture, id: SharedKeyId, why: &str) {
    let error = fixture.service.public_key_text(id).unwrap_err();
    assert_eq!(error.code(), ResultCode::PublicKeyUnavailable, "{why}");
    assert_eq!(
        error.code().failure_class(),
        Some(FailureClass::Blocked),
        "{why}"
    );
    assert_eq!(error.scope, Scope::default(), "{why}");
    assert!(error.recovery.is_empty(), "{why}");
    assert_failure_has_no_secret("key public", &error);
}

#[test]
fn a_public_key_file_that_cannot_be_returned_is_unavailable() {
    let fixture = fixture();
    let keys = &fixture.keys;
    let id = keys.imported.id;
    let public = keys.imported.public_key_path.clone().unwrap();
    let text = credentials::public_fixture_text();
    let line = text.trim_end();

    assert_public_key_unavailable(&fixture, keys.without_public.id, "no public key path");

    // 16 KiB is the largest file read; one byte more is refused.
    let padding = |length: usize| "x".repeat(length - line.len() - 1);
    fs::write(&public, format!("{line} {}", padding(16 * 1024))).unwrap();
    assert_eq!(fs::metadata(&public).unwrap().len(), 16 * 1024);
    fixture.service.public_key_text(id).unwrap();
    fs::write(&public, format!("{line} {}", padding(16 * 1024 + 1))).unwrap();
    assert_public_key_unavailable(&fixture, id, "oversized");

    let mut not_utf8 = line.as_bytes().to_vec();
    not_utf8.extend_from_slice(b" \xff\xfe\n");
    fs::write(&public, not_utf8).unwrap();
    assert_public_key_unavailable(&fixture, id, "not UTF-8");

    fs::write(&public, "this is not a public key\n").unwrap();
    assert_public_key_unavailable(&fixture, id, "not a public key");
    fs::write(&public, "").unwrap();
    assert_public_key_unavailable(&fixture, id, "empty");

    fs::remove_file(&public).unwrap();
    assert_public_key_unavailable(&fixture, id, "missing");
    fs::create_dir(&public).unwrap();
    assert_public_key_unavailable(&fixture, id, "a directory");
    fs::remove_dir(&public).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if permissions_bind(fixture.keys.files.path()) {
            fs::write(&public, &text).unwrap();
            fs::set_permissions(&public, fs::Permissions::from_mode(0o000)).unwrap();
            assert_public_key_unavailable(&fixture, id, "unreadable");
            fs::remove_file(&public).unwrap();
        }
        // A FIFO with no writer must not block the read.
        let name = std::ffi::CString::new(public.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let started = Instant::now();
        assert_public_key_unavailable(&fixture, id, "a FIFO");
        assert!(started.elapsed() < NOT_BLOCKED);
        fs::remove_file(&public).unwrap();
    }

    // The registration itself is unchanged and still readable.
    fs::write(&public, &text).unwrap();
    assert!(
        fixture
            .service
            .public_key_text(id)
            .unwrap()
            .matches_registration
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_private_key_in_the_public_key_file_is_never_returned() {
    let fixture = fixture();
    let keys = &fixture.keys;
    let id = keys.imported.id;
    let public = keys.imported.public_key_path.clone().unwrap();
    let line = credentials::public_fixture_text().trim_end().to_owned();
    let private = String::from_utf8(credentials::private_fixture_bytes()).unwrap();
    let generated_private = fs::read_to_string(&keys.generated.private_key_path).unwrap();
    assert!(private.contains(credentials::PRIVATE_KEY_SENTINEL));

    let contents = [
        private.clone(),
        generated_private,
        // A real public key first, so that the file begins as one.
        format!("{line}\n{private}"),
        format!("{line}\r\n{private}"),
        format!("{line} comment\n\n{private}\n"),
        format!("{private}\n{line}\n"),
    ];
    for contents in contents {
        fs::write(&public, &contents).unwrap();
        assert_public_key_unavailable(&fixture, id, &contents);
    }

    // And where the registration names the private key file as its public.
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let outcome = service
        .register_shared_key(manyhands::repository::RegisterSharedKeyRequest {
            label: "Same file".to_owned(),
            ownership: manyhands::repository::SharedKeyOwnership::Imported,
            private_key_path: keys.without_public.private_key_path.clone(),
            public_key_path: Some(keys.without_public.private_key_path.clone()),
        })
        .unwrap();
    let manyhands::repository::RegisterSharedKeyOutcome::Registered(same) = outcome else {
        panic!("not registered");
    };
    let error = service.public_key_text(same.id).unwrap_err();
    assert_eq!(error.code(), ResultCode::PublicKeyUnavailable);
    assert_failure_has_no_secret("key public", &error);
    assert_no_secret("key show", &service.show_key(same.id).unwrap());
    assert_git_transport_uninitialized();
}

fn pin(host: &str, port: u16, algorithm: &str, sha256: &str, reapproval: bool) -> HostPinDto {
    HostPinDto {
        host: host.to_owned(),
        port,
        algorithm: algorithm.to_owned(),
        sha256: sha256.to_owned(),
        reapproval_required: reapproval,
    }
}

/// Pins stored out of order, with ports that sort differently as text.
fn pin_hosts(fixture: &Fixture) {
    fixture.pin("b.example", 22, "ssh-ed25519", OTHER_FINGERPRINT);
    fixture.pin("a.example", 10022, "ssh-rsa", PUBLIC_FIXTURE_FINGERPRINT);
    fixture.pin("::1", 22, "ssh-ed25519", PUBLIC_FIXTURE_FINGERPRINT);
    fixture.pin("a.example", 2222, "ssh-ed25519", OTHER_FINGERPRINT);
    fixture.pin(
        "a.example",
        22,
        "ecdsa-sha2-nistp256",
        PUBLIC_FIXTURE_FINGERPRINT,
    );
}

#[test]
fn host_pins_list_by_host_and_then_port() {
    let fixture = fixture();
    assert!(fixture.service.list_host_pins().unwrap().items.is_empty());
    assert!(fixture.service.list_host_pins().unwrap().complete);
    pin_hosts(&fixture);

    let list = fixture.service.list_host_pins().unwrap();

    assert!(list.complete);
    assert_eq!(
        list.items,
        [
            pin("::1", 22, "ssh-ed25519", PUBLIC_FIXTURE_FINGERPRINT, false),
            pin(
                "a.example",
                22,
                "ecdsa-sha2-nistp256",
                PUBLIC_FIXTURE_FINGERPRINT,
                false
            ),
            pin("a.example", 2222, "ssh-ed25519", OTHER_FINGERPRINT, false),
            pin(
                "a.example",
                10022,
                "ssh-rsa",
                PUBLIC_FIXTURE_FINGERPRINT,
                false
            ),
            pin("b.example", 22, "ssh-ed25519", OTHER_FINGERPRINT, false),
        ]
    );
    assert_no_secret("host list", &list);
    assert_git_transport_uninitialized();
}

#[test]
fn a_host_is_inspected_by_its_authority_as_a_pin_stores_it() {
    let fixture = fixture();
    pin_hosts(&fixture);
    let service = &fixture.service;

    let expected = pin("a.example", 2222, "ssh-ed25519", OTHER_FINGERPRINT, false);
    assert_eq!(
        service.inspect_host(&authority("a.example", 2222)).unwrap(),
        expected
    );
    // The host is stored in lower case, and compared that way.
    assert_eq!(
        service.inspect_host(&authority("A.Example", 2222)).unwrap(),
        expected
    );
    // An address is stored in its canonical text.
    let loopback = pin("::1", 22, "ssh-ed25519", PUBLIC_FIXTURE_FINGERPRINT, false);
    for host in ["::1", "0:0:0:0:0:0:0:1"] {
        assert_eq!(
            service.inspect_host(&authority(host, 22)).unwrap(),
            loopback
        );
    }

    let unknown = [
        authority("a.example", 23),
        authority("c.example", 22),
        authority("example", 22),
        // None of these can be the authority of a pin.
        authority("a.example", 0),
        authority("", 22),
        authority("a.example/x", 2222),
        authority("git@a.example", 2222),
        authority("a.example:2222", 22),
        authority("a.example ", 2222),
        authority("a.example\n", 2222),
        authority("%", 22),
        authority("a.example' OR '1'='1", 2222),
    ];
    for authority in unknown {
        let error = service.inspect_host(&authority).unwrap_err();
        assert_eq!(error.code(), ResultCode::AuthorityNotFound, "{authority:?}");
        assert_eq!(error.code().failure_class(), Some(FailureClass::Input));
        assert_eq!(error.scope, Scope::default());
        assert!(error.recovery.is_empty());
    }
    assert_git_transport_uninitialized();
}

#[test]
fn the_reapproval_marker_is_reflected_in_every_pin() {
    let fixture = fixture();
    pin_hosts(&fixture);
    let target = authority("b.example", 22);
    assert!(
        !fixture
            .service
            .inspect_host(&target)
            .unwrap()
            .reapproval_required
    );

    credentials::require_host_reapproval(fixture.data.path());

    let list = fixture.service.list_host_pins().unwrap();
    assert_eq!(list.items.len(), 5);
    assert!(list.items.iter().all(|pin| pin.reapproval_required));
    assert_eq!(
        fixture.service.inspect_host(&target).unwrap(),
        pin("b.example", 22, "ssh-ed25519", OTHER_FINGERPRINT, true)
    );
    // The marker is read, never written or removed.
    let marker = fixture
        .data
        .path()
        .join("ssh-host-trust-reapproval-required");
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"manyhands SSH host trust reapproval required v1\n"
    );

    // A marker that is not the marker is a failure, not a guess.
    fs::write(&marker, b"something else").unwrap();
    for error in [
        fixture.service.list_host_pins().unwrap_err(),
        fixture.service.inspect_host(&target).unwrap_err(),
    ] {
        assert_eq!(error.code(), ResultCode::InternalError);
    }
    assert_eq!(fs::read(&marker).unwrap(), b"something else");
    assert_git_transport_uninitialized();
}

#[test]
fn host_reads_succeed_under_the_shared_lock_and_are_busy_under_the_exclusive_lock() {
    let fixture = fixture();
    pin_hosts(&fixture);
    let target = authority("a.example", 22);
    let expected = fixture.service.list_host_pins().unwrap();

    let shared = fixture.hold_index_lock(LeaseKind::CacheRead);
    let started = Instant::now();
    assert_eq!(fixture.service.list_host_pins().unwrap(), expected);
    assert_eq!(
        fixture.service.inspect_host(&target).unwrap(),
        expected.items[1]
    );
    assert!(started.elapsed() < NOT_BLOCKED);
    shared.release();

    let exclusive = fixture.hold_index_lock(LeaseKind::CacheWrite);
    let started = Instant::now();
    let busy = [
        fixture.service.list_host_pins().unwrap_err(),
        fixture.service.inspect_host(&target).unwrap_err(),
        fixture
            .service
            .inspect_host(&authority("c.example", 22))
            .unwrap_err(),
        // Busy even for an authority that no pin can have.
        fixture.service.inspect_host(&authority("", 0)).unwrap_err(),
    ];
    assert!(started.elapsed() < NOT_BLOCKED);
    for error in &busy {
        assert_eq!(error.code(), ResultCode::Busy);
        assert!(error.recovery.is_empty());
        let envelope = error.to_envelope::<Value>("host list");
        assert_eq!(envelope.outcome, Outcome::Error);
    }

    exclusive.release();
    assert_eq!(fixture.service.list_host_pins().unwrap(), expected);
    assert_git_transport_uninitialized();
}

#[test]
fn credential_reads_report_a_degraded_index_as_unavailable() {
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let id = SharedKeyId::new();

    let errors = [
        service.list_keys().unwrap_err(),
        service.show_key(id).unwrap_err(),
        service.public_key_text(id).unwrap_err(),
        service.list_host_pins().unwrap_err(),
        service
            .inspect_host(&authority("a.example", 22))
            .unwrap_err(),
    ];

    for error in &errors {
        assert_eq!(error.code(), ResultCode::IndexUnavailable);
        assert_eq!(error.recovery.len(), 1);
        assert_eq!(error.recovery[0].action, "index.rebuild");
    }
    assert_git_transport_uninitialized();
}

#[test]
fn credential_reads_change_neither_the_index_nor_the_data_directory() {
    let fixture = fixture();
    pin_hosts(&fixture);
    credentials::require_host_reapproval(fixture.data.path());
    let snapshot = |directory: &Path| {
        let mut entries: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            // The lock file is opened by every lease; its bytes are not data.
            .filter(|path| !path.to_str().unwrap().ends_with(".lock"))
            .map(|path| {
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        entries.sort();
        entries
    };
    let names = |entries: &[(PathBuf, Vec<u8>)]| -> Vec<PathBuf> {
        entries.iter().map(|(path, _)| path.clone()).collect()
    };
    // The first read-only connection after a writer has closed recreates
    // SQLite's empty journal files, as every read session does; that is the
    // session's own behavior, so it happens before the snapshot.
    fixture.service.list_keys().unwrap();
    let before = snapshot(fixture.data.path());
    assert!(before.len() > 2);
    let files_before = credentials::key_file_states(&fixture.keys.key_files());

    every_key_read(&fixture);
    fixture.service.list_host_pins().unwrap();
    fixture
        .service
        .inspect_host(&authority("a.example", 22))
        .unwrap();
    fixture
        .service
        .inspect_host(&authority("nowhere.example", 22))
        .unwrap_err();

    let after = snapshot(fixture.data.path());
    assert_eq!(names(&before), names(&after));
    for ((path, before), (_, after)) in before.iter().zip(&after) {
        assert!(before == after, "{path:?} changed");
    }
    assert_eq!(
        credentials::key_file_states(&fixture.keys.key_files()),
        files_before
    );
    assert_git_transport_uninitialized();
}

// A row this build cannot read is a failure of the registry, never a guess
// and never a partial list.
#[test]
fn a_stored_row_that_cannot_be_read_is_an_internal_error() {
    let fixture = fixture();
    pin_hosts(&fixture);
    let index = fixture.data.path().join("manyhands.sqlite3");
    let connection = rusqlite::Connection::open(&index).unwrap();
    connection
        .execute(
            "UPDATE shared_ssh_keys SET label = '  ' WHERE id = ?1",
            [fixture.keys.without_public.id.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE ssh_host_pins SET port = 70000 WHERE host = 'b.example'",
            [],
        )
        .unwrap();
    drop(connection);

    let errors = [
        fixture.service.list_keys().unwrap_err(),
        fixture
            .service
            .show_key(fixture.keys.imported.id)
            .unwrap_err(),
        fixture
            .service
            .public_key_text(fixture.keys.imported.id)
            .unwrap_err(),
        fixture.service.list_host_pins().unwrap_err(),
        fixture
            .service
            .inspect_host(&authority("b.example", 22))
            .unwrap_err(),
    ];

    for error in &errors[..4] {
        assert_eq!(error.code(), ResultCode::InternalError);
        assert!(error.recovery.is_empty());
        assert_failure_has_no_secret("key list", error);
        let text = serde_json::to_string(&error.to_envelope::<Value>("key list")).unwrap();
        assert!(!text.contains(fixture.data.path().to_str().unwrap()));
    }
    // The pin that cannot be read has no authority a caller can name.
    assert_eq!(errors[4].code(), ResultCode::AuthorityNotFound);
    assert_git_transport_uninitialized();
}
