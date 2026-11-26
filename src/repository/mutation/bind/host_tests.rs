//! Which remote a host command is about (Wave 06 Cycle 02, design Part C):
//! the publication remote of the configuration file, or, when the file
//! names none, the remote of the pending setup.

use std::path::{Path, PathBuf};

use git2::{Repository, Signature, Time};

use super::{Host, SshAuthority};
use crate::{
    repository::{
        CommitIdentity, EnableRepositoryRequest, OperationId, REGISTRY_FILE, RepositoryService,
        RequestId,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        open_registry,
        setup::record::{self, SetupKind, SetupRecord},
    },
    results::ResultCode,
};

const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const FILE_URL: &str = "ssh://git@file.example.invalid/team/repo.git";
const SETUP_URL: &str = "ssh://git@setup.example.invalid:2222/fork/repo.git";
const CONFIGURATION: &str = "format_version = 1\nprimary_branch = \"main\"\n";

struct Fixture {
    _directory: tempfile::TempDir,
    data: tempfile::TempDir,
    root: PathBuf,
    service: RepositoryService,
}

struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a call that stops in its preparation asks for no passphrase")
    }
}

impl Fixture {
    /// An enabled repository with two remotes, `origin` at `FILE_URL` and
    /// `upstream` at `SETUP_URL`, and no publication remote selected.
    fn new() -> Self {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let root = directory.path().to_owned();
        let data = tempfile::tempdir().unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(&root, &options).unwrap();
        std::fs::write(root.join("fixture.txt"), "fixture\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            Signature::new("Fixture", "fixture@example.invalid", &Time::new(0, 0)).unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[])
            .unwrap();
        repository.remote("origin", FILE_URL).unwrap();
        repository.remote("upstream", SETUP_URL).unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: Some(CommitIdentity {
                    name: "Fixture".into(),
                    email: "fixture@example.invalid".into(),
                }),
                operation_id: OperationId::new(),
            })
            .unwrap();
        Self {
            _directory: directory,
            data,
            root,
            service,
        }
    }

    /// The configuration file as the working tree has it: `None` is no file.
    fn file(&self, text: Option<&str>) {
        let path = self.root.join(".manyhands/config.toml");
        match text {
            Some(text) => std::fs::write(path, text).unwrap(),
            None => std::fs::remove_file(path).unwrap(),
        }
    }

    /// A pending setup of the remote `upstream`.
    fn setup(&self) {
        let mut db = open_registry(&self.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap();
        let id: i64 = db
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        let tx = db.transaction().unwrap();
        record::insert(
            &tx,
            id,
            &SetupRecord {
                setup_id: SETUP.parse().unwrap(),
                kind: SetupKind::Enable,
                remote_name: "upstream".into(),
                primary_branch: "main".into(),
                endpoint_digest: [7; 32],
                configuration: b"format_version = 2\nprimary_branch = \"main\"\n".to_vec(),
                base_configuration_oid: None,
                base_oid: None,
                commit_oid: None,
                published_oid: None,
                request_id: RequestId::new(),
                created_at: 1_700_000_000,
            },
        )
        .unwrap();
        tx.commit().unwrap();
    }
}

fn authority(host: &str, port: u16) -> SshAuthority {
    SshAuthority {
        host: host.into(),
        port,
    }
}

/// What a read of the host answers: the remote it is about, whether that
/// remote is the setup's, the host, and where the remote is used for it.
type Read = Result<(String, bool, SshAuthority, bool, bool), ResultCode>;

fn read(f: &Fixture, authority: Option<&SshAuthority>) -> Read {
    Host::read(&f.service, &f.root, authority)
        .map(|host| {
            (
                host.remote,
                host.setup.is_some(),
                host.authority,
                host.fetch.is_some(),
                host.push.is_some(),
            )
        })
        .map_err(|answer| answer.code)
}

/// A row of the test: what the file holds, a pending setup, the host asked
/// for, and the answer.
type Row<'a> = (
    &'a str,
    Option<&'a str>,
    bool,
    Option<&'a SshAuthority>,
    Read,
);

fn of(remote: &str, setup: bool, host: &SshAuthority, used: bool) -> Read {
    Ok((remote.to_owned(), setup, host.clone(), used, used))
}

/// `host observe` and `host approve` read the file's publication remote
/// when it names one and the pending setup's otherwise; given a host, the
/// remote that names it, the file's first. A repository with neither is
/// refused as it was before setups: `invalid_configuration` for no file,
/// `publication_remote_required` for a file with no remote.
#[test]
fn host_read_takes_the_remote_of_design_part_c() {
    let file_host = authority("file.example.invalid", 22);
    let setup_host = authority("setup.example.invalid", 2222);
    let unrelated = authority("elsewhere.example.invalid", 22);
    let origin = "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
    // (what the file holds, a pending setup, the host asked for, the answer)
    let rows: Vec<Row<'_>> = vec![
        // Today's reads: the file's remote, with or without a setup.
        (
            "a remote, no setup",
            Some(origin),
            false,
            None,
            of("origin", false, &file_host, true),
        ),
        (
            "a remote, no setup, its host",
            Some(origin),
            false,
            Some(&file_host),
            of("origin", false, &file_host, true),
        ),
        (
            "a remote, a setup",
            Some(origin),
            true,
            None,
            of("origin", false, &file_host, true),
        ),
        (
            "a remote, a setup, the file's host",
            Some(origin),
            true,
            Some(&file_host),
            of("origin", false, &file_host, true),
        ),
        // A host only the setup's remote names.
        (
            "a remote, a setup, the setup's host",
            Some(origin),
            true,
            Some(&setup_host),
            of("upstream", true, &setup_host, true),
        ),
        // A host neither remote names: the file's remote, which does not use it.
        (
            "a remote, a setup, another host",
            Some(origin),
            true,
            Some(&unrelated),
            of("origin", false, &unrelated, false),
        ),
        // The file names no remote: the setup's.
        (
            "no remote, a setup",
            Some(CONFIGURATION),
            true,
            None,
            of("upstream", true, &setup_host, true),
        ),
        (
            "no remote, a setup, its host",
            Some(CONFIGURATION),
            true,
            Some(&setup_host),
            of("upstream", true, &setup_host, true),
        ),
        (
            "no remote, a setup, another host",
            Some(CONFIGURATION),
            true,
            Some(&unrelated),
            of("upstream", true, &unrelated, false),
        ),
        (
            "no file, a setup",
            None,
            true,
            None,
            of("upstream", true, &setup_host, true),
        ),
        (
            "no file, a setup, its host",
            None,
            true,
            Some(&setup_host),
            of("upstream", true, &setup_host, true),
        ),
        // Neither: as before setups.
        (
            "no file, no setup",
            None,
            false,
            None,
            Err(ResultCode::InvalidConfiguration),
        ),
        (
            "no file, no setup, a host",
            None,
            false,
            Some(&file_host),
            Err(ResultCode::InvalidConfiguration),
        ),
        (
            "no remote, no setup",
            Some(CONFIGURATION),
            false,
            None,
            Err(ResultCode::PublicationRemoteRequired),
        ),
        (
            "no remote, no setup, a host",
            Some(CONFIGURATION),
            false,
            Some(&file_host),
            Err(ResultCode::PublicationRemoteRequired),
        ),
        // A file that cannot be read is not replaced by a setup.
        (
            "an invalid file, a setup",
            Some("not = [valid"),
            true,
            None,
            Err(ResultCode::InvalidConfiguration),
        ),
    ];
    let (mut actual, mut expected) = (Vec::new(), Vec::new());
    for (name, file, setup, host, answer) in rows {
        let f = Fixture::new();
        f.file(file);
        if setup {
            f.setup();
        }
        actual.push((name, read(&f, host)));
        expected.push((name, answer));
    }
    assert_eq!(actual, expected);
}

/// A host observation of a repository with a pending setup and no
/// publication remote goes through the setup's remote: it reaches the
/// transport, which stops at the missing key, before any connection, where
/// the file's missing remote would have stopped it at the read.
#[test]
fn host_observe_on_a_pending_setup_without_a_key_is_no_selected_key() {
    let _initialized = crate::runtime::pretend_initialized_for_preparation();
    let f = Fixture::new();
    f.file(None);
    f.setup();
    let envelope =
        f.service
            .observe_host(&f.root, None, &mut SessionCredentials::new(NoPassphrase));
    assert_eq!(envelope.code, ResultCode::NoSelectedKey);
    // The same repository with no setup stops at the read, as it did.
    let f = Fixture::new();
    f.file(None);
    let envelope =
        f.service
            .observe_host(&f.root, None, &mut SessionCredentials::new(NoPassphrase));
    assert_eq!(envelope.code, ResultCode::InvalidConfiguration);
}
