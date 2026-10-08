//! Key registrations and host pins for the credential read tests.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use manyhands::repository::{
    OperationId, REGISTRY_FILE, RegisterSharedKeyOutcome, RegisterSharedKeyRequest,
    RepositoryService, SharedKeyOwnership, SharedKeyRegistration, SharedKeySelectionOutcome,
    keys::{
        GenerateSharedKeyOutcome, GenerateSharedKeyRequest, KeyProtection, KeyStore,
        SecretPassphrase,
    },
    transport::{HostKeyIdentity, SshAuthority},
};

/// The two markers inside `tests/shared_key_registry_private_fixture`.
pub const PRIVATE_KEY_SENTINEL: &str = "CYCLE01_PRIVATE_SENTINEL_DO_NOT_PERSIST";
pub const PRIVATE_FIXTURE_PASSPHRASE_SENTINEL: &str = "CYCLE01_PASSPHRASE_SENTINEL_DO_NOT_PERSIST";
/// The passphrase that protects the generated key.
pub const GENERATED_PASSPHRASE_SENTINEL: &str = "READ-PASSPHRASE-SENTINEL-5b1e";
/// What no credential read may return.
pub const SECRET_SENTINELS: [&str; 4] = [
    PRIVATE_KEY_SENTINEL,
    PRIVATE_FIXTURE_PASSPHRASE_SENTINEL,
    GENERATED_PASSPHRASE_SENTINEL,
    "PRIVATE KEY",
];

/// The fingerprint of `tests/shared_key_registry_public_fixture.pub`.
pub const PUBLIC_FIXTURE_FINGERPRINT: &str = "SHA256:kmYcvdi2GkPeWxB6XLjrZB8JHsy2Hm8luHMFp9GMvqk";
/// A second well-formed fingerprint, for a host key.
pub const OTHER_FINGERPRINT: &str = "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU";

/// The marker's file name, for the tests that damage or compare the file.
/// The marker itself is only ever written by the service.
pub const REAPPROVAL_MARKER_FILE: &str = "ssh-host-trust-reapproval-required";

fn fixture_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
}

pub fn public_fixture_text() -> String {
    fs::read_to_string(fixture_file("shared_key_registry_public_fixture.pub")).unwrap()
}

pub fn private_fixture_bytes() -> Vec<u8> {
    fs::read(fixture_file("shared_key_registry_private_fixture")).unwrap()
}

/// Three registrations, in this order: an imported key with a public key
/// file, a generated passphrase-protected key, which is the selected one,
/// and an imported key with no public key file.
pub struct RegisteredKeys {
    pub imported: SharedKeyRegistration,
    pub generated: SharedKeyRegistration,
    pub without_public: SharedKeyRegistration,
    /// Holds the imported files, which are copies the test may replace.
    pub files: tempfile::TempDir,
    pub home: tempfile::TempDir,
}

impl RegisteredKeys {
    /// Every key file of the three registrations, private files first.
    pub fn key_files(&self) -> Vec<PathBuf> {
        let registrations = [&self.imported, &self.generated, &self.without_public];
        registrations
            .iter()
            .map(|registration| registration.private_key_path.clone())
            .chain(
                registrations
                    .iter()
                    .filter_map(|registration| registration.public_key_path.clone()),
            )
            .collect()
    }

    pub fn private_key_files(&self) -> Vec<PathBuf> {
        [&self.imported, &self.generated, &self.without_public]
            .map(|registration| registration.private_key_path.clone())
            .to_vec()
    }
}

pub fn register_keys(service: &RepositoryService) -> RegisteredKeys {
    let files = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let directory = fs::canonicalize(files.path()).unwrap();
    let private = directory.join("imported");
    let public = directory.join("imported.pub");
    let lone = directory.join("lone");
    fs::write(&private, private_fixture_bytes()).unwrap();
    fs::write(&public, public_fixture_text()).unwrap();
    fs::write(&lone, private_fixture_bytes()).unwrap();

    let register = |label: &str, private: &Path, public: Option<&Path>| {
        let outcome = service
            .register_shared_key(RegisterSharedKeyRequest {
                label: label.to_owned(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: private.to_owned(),
                public_key_path: public.map(Path::to_owned),
            })
            .unwrap();
        match outcome {
            RegisterSharedKeyOutcome::Registered(registration) => registration,
            RegisterSharedKeyOutcome::SourceAlreadyRegistered { .. } => panic!("registered twice"),
        }
    };
    let imported = register("Imported key", &private, Some(&public));
    let store = KeyStore::for_home(home.path()).unwrap();
    let generated = match service
        .generate_shared_key(
            &store,
            GenerateSharedKeyRequest {
                operation_id: OperationId::new(),
                label: "Generated key".to_owned(),
                protection: KeyProtection::Passphrase(
                    SecretPassphrase::new(GENERATED_PASSPHRASE_SENTINEL.to_owned()).unwrap(),
                ),
            },
        )
        .unwrap()
    {
        GenerateSharedKeyOutcome::Created(registration) => registration,
        _ => panic!("the key was not generated"),
    };
    let without_public = register("Key without a public file", &lone, None);
    let generated = match service.select_shared_key(generated.id).unwrap() {
        SharedKeySelectionOutcome::Selected(registration) => registration,
        _ => panic!("the key was not selected"),
    };

    RegisteredKeys {
        imported,
        generated,
        without_public,
        files,
        home,
    }
}

/// What a read must leave as it found it: a key file's kind, bytes,
/// permissions and modification time. `None` when there is nothing there.
#[derive(Debug, PartialEq, Eq)]
pub struct KeyFileState {
    pub path: PathBuf,
    pub entry: Option<(bool, Vec<u8>, u32, SystemTime)>,
}

pub fn key_file_states(paths: &[PathBuf]) -> Vec<KeyFileState> {
    paths
        .iter()
        .map(|path| KeyFileState {
            path: path.clone(),
            entry: fs::symlink_metadata(path).ok().map(|metadata| {
                // Read with the metadata alone where the file cannot be
                // opened, which is the point of some of these tests.
                let bytes = if metadata.is_file() {
                    fs::read(path).unwrap_or_default()
                } else {
                    Vec::new()
                };
                (
                    metadata.is_file(),
                    bytes,
                    permission_bits(&metadata),
                    metadata.modified().unwrap(),
                )
            }),
        })
        .collect()
}

#[cfg(unix)]
fn permission_bits(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn permission_bits(metadata: &fs::Metadata) -> u32 {
    u32::from(metadata.permissions().readonly())
}

/// Pins a host through the code an approved operation uses.
pub fn pin_host(service: &RepositoryService, host: &str, port: u16, algorithm: &str, sha256: &str) {
    service
        .approve_host_pin_for_testing(
            &SshAuthority {
                host: host.to_owned(),
                port,
            },
            &HostKeyIdentity {
                algorithm: algorithm.to_owned(),
                sha256: sha256.to_owned(),
            },
        )
        .unwrap();
}

/// Leaves the marker a recovery of the pin registry leaves, as it does.
pub fn require_host_reapproval(service: &RepositoryService) {
    service.require_host_reapproval_for_testing().unwrap();
}

/// A writable connection to the index, for a test that damages a row.
pub fn index_connection(data_directory: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(data_directory.join(REGISTRY_FILE)).unwrap()
}

/// Every file in the data directory but the lock files, whose bytes are not
/// data, with its bytes.
pub fn data_directory_files(data_directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(data_directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .map(|entry| {
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .filter(|(name, _)| !name.ends_with(".lock"))
        .collect();
    files.sort();
    files
}

/// What reads may do to the data directory between two snapshots: nothing,
/// except that the first read-only connection after a writer has closed
/// creates SQLite's two journal files. The index and every other file that
/// was there is byte for byte what it was.
pub fn assert_reads_left_the_data_directory(
    before: &[(String, Vec<u8>)],
    after: &[(String, Vec<u8>)],
) {
    let journal = [
        format!("{REGISTRY_FILE}-shm"),
        format!("{REGISTRY_FILE}-wal"),
    ];
    assert!(before.iter().any(|(name, _)| name == REGISTRY_FILE));
    for (name, bytes) in before {
        let now = after.iter().find(|(after, _)| after == name);
        let Some((_, now)) = now else {
            panic!("{name} was removed");
        };
        assert!(now == bytes, "{name} changed");
    }
    for (name, _) in after {
        assert!(
            before.iter().any(|(before, _)| before == name) || journal.contains(name),
            "{name} appeared"
        );
    }
}
