//! A remote call reads the shared keys through one registry connection
//! (design decision 36), and each of its reads still reads the current
//! rows. No network: the call's steps run on a handle that never connected,
//! which refuses each listing and transfer after its rechecks.
use super::*;
use crate::repository::{
    REGISTRY_FILE,
    discovery::migrations_for_testing,
    key_reads,
    remote::{RemoteRefPlan, SynchronizationTarget},
    transport::callbacks::CallbackAttempt,
};
use std::{fs, path::PathBuf};

const CONFIGURATION: &str =
    "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
const URL: &str = "ssh://fixture@example.invalid/repository.git";

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    key: PathBuf,
    service: RepositoryService,
    id: SharedKeyId,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("repository");
        let data = directory.path().join("data");
        let repository = git2::Repository::init(&root).unwrap();
        fs::create_dir_all(root.join(".manyhands")).unwrap();
        fs::write(root.join(".manyhands/config.toml"), CONFIGURATION).unwrap();
        repository.remote("origin", URL).unwrap();
        let service = RepositoryService::open_at(&data).unwrap();
        let key = data.join("selected-key");
        let id = register(&service, &key, b"fixture source identity only");
        service.select_shared_key(id).unwrap();
        Self {
            _directory: directory,
            root,
            data,
            key,
            service,
            id,
        }
    }

    fn plan(&self) -> RemoteRefPlan {
        RemoteRefPlan::from_configuration("origin", "main").unwrap()
    }

    /// What `with_authenticated_remote_policy_expected` prepares.
    fn prepare(&self, direction: SshDirection) -> (PreparedSshAttempt, SshTransportError) {
        let mut context = SshTransportError {
            root: self.root.clone(),
            remote_name: String::new(),
            direction,
            selected_key_id: None,
            authority: None,
            kind: SshTransportErrorKind::ConfigurationInvalid,
        };
        let prepared = self
            .service
            .prepare_ssh(
                VerifySshTransportRequest {
                    root: self.root.clone(),
                    direction,
                    approval: None,
                },
                &mut context,
            )
            .unwrap();
        (prepared, context)
    }

    fn expectation(&self, prepared: &PreparedSshAttempt) -> SshScopeExpectation {
        SshScopeExpectation::new(
            prepared.endpoint.clone(),
            Some((
                self.id,
                self.key.clone(),
                Some(KeySourceToken::observe(&self.key).unwrap()),
            )),
        )
    }

    /// Another file put in the registry's place, as a rebuild puts one
    /// there: the registry and its sidecars set aside, as a corrupt one is,
    /// and a copy of its rows, with `edit` made to them, in its place.
    /// Unix only: Windows refuses to rename a file SQLite holds open.
    #[cfg(unix)]
    fn replace_registry(&self, edit: &str) {
        let registry = self.data.join(REGISTRY_FILE);
        let copy = self.data.join("replacement.sqlite3");
        let source = rusqlite::Connection::open(&registry).unwrap();
        source
            .execute("VACUUM INTO ?1", [copy.to_str().unwrap()])
            .unwrap();
        drop(source);
        rusqlite::Connection::open(&copy)
            .unwrap()
            .execute_batch(edit)
            .unwrap();
        crate::repository::replace_corrupt_registry(&registry, &self.root).unwrap();
        fs::rename(&copy, &registry).unwrap();
    }
}

fn register(service: &RepositoryService, key: &std::path::Path, source: &[u8]) -> SharedKeyId {
    fs::write(key, source).unwrap();
    match service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "fixture".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: key.into(),
            public_key_path: None,
        })
        .unwrap()
    {
        RegisterSharedKeyOutcome::Registered(registration) => registration.id,
        other => panic!("{other:?}"),
    }
}

fn identity(byte: u8) -> HostKeyIdentity {
    let public = ssh_key::private::Ed25519Keypair::from_seed(&[byte; 32]);
    let public = ssh_key::PublicKey::from(public.public);
    HostKeyIdentity {
        algorithm: public.algorithm().to_string(),
        sha256: public.fingerprint(ssh_key::HashAlg::Sha256).to_string(),
    }
}

/// A step of a call refused by its never-connected handle only after the
/// step's rechecks passed.
fn after_its_rechecks<T: std::fmt::Debug>(step: Result<T, SshTransportError>) {
    assert_eq!(
        step.unwrap_err().kind,
        SshTransportErrorKind::ProtocolFailure
    );
}

#[test]
fn a_remote_call_opens_the_registry_once_for_its_key_reads() {
    let f = Fixture::new();
    let plan = f.plan();
    // Outside a remote call, each key read opens the registry (Task 20).
    let before = key_reads::counts();
    f.service.list_shared_keys().unwrap();
    let configuration = f.service.observation_configuration(&f.root, &plan).unwrap();
    let after = key_reads::counts();
    assert_eq!(
        (after.opens - before.opens, after.opens_in_calls),
        (2, before.opens_in_calls)
    );
    // Each remote call, as a poll and a sync make it (a comment
    // publication's calls are a sync's): its preparation, the rechecks
    // before and after it connects, then its steps on the connection. Over
    // SSH, E-68, E-69 and E-70 count the calls of a real sync, comment
    // publication and poll.
    for call in ["poll", "sync"] {
        let before = key_reads::counts();
        let direction = if call == "poll" {
            SshDirection::Fetch
        } else {
            SshDirection::Push
        };
        let (prepared, context) = f.prepare(direction);
        let expectation = f.expectation(&prepared);
        f.service.recheck_ssh(&prepared, &context).unwrap();
        f.service.recheck_ssh(&prepared, &context).unwrap();
        let repository = git2::Repository::open(&f.root).unwrap();
        let mut handle = repository
            .remote_anonymous(&prepared.endpoint.connection_url)
            .unwrap();
        let attempt = CallbackAttempt::default();
        let mut remote = AuthenticatedSshRemote::new(
            &mut handle,
            &attempt,
            &f.service,
            &prepared,
            &context,
            None,
            None,
            identity(1),
        );
        if call == "poll" {
            after_its_rechecks(remote.fetch_poll_set(&plan, |_| true));
        } else {
            for _ in 0..2 {
                // The action checks before and after each step, a step,
                // and the push's decision, which reads the configuration.
                remote.require_action_expectation(&expectation).unwrap();
                let read =
                    f.service
                        .observation_configuration_in(remote.call_registry(), &f.root, &plan);
                assert!(read.unwrap() == configuration);
                after_its_rechecks(remote.fresh_advertisement());
                after_its_rechecks(remote.push_exact(&plan, &SynchronizationTarget::Primary));
            }
        }
        drop(remote);
        drop(prepared);
        let after = key_reads::counts();
        assert_eq!(after.calls - before.calls, 1, "{call}");
        assert_eq!(after.opens - before.opens, 1, "{call}");
        assert_eq!(after.opens_in_calls - before.opens_in_calls, 1, "{call}");
    }
}

/// The rechecks of one call read the current rows through its connection:
/// a change made between two of its steps is refused at the next one,
/// with the answer a recheck on a connection of its own gives. The call's
/// configuration read sees it too.
#[test]
fn a_change_between_two_steps_of_one_call_is_refused_at_its_next_recheck() {
    use SshTransportErrorKind::*;
    let rows: &[(&str, Option<SshTransportErrorKind>, bool)] = &[
        // (change, what the next recheck answers, the configuration moved)
        ("nothing", None, false),
        ("key deselected", Some(SelectionChanged), true),
        ("another key selected", Some(SelectionChanged), true),
        ("key file replaced", Some(KeySourceChanged), true),
        ("key file removed", Some(KeySourceChanged), true),
        ("key removed", Some(SelectionChanged), true),
        ("host pin changed", Some(HostTrustChanged), false),
        #[cfg(unix)]
        ("registry file replaced", None, false),
        #[cfg(unix)]
        (
            "registry file replaced, key deselected",
            Some(SelectionChanged),
            true,
        ),
        // Named edit E-71: Windows refuses to rename the registry while a
        // call holds it open (SQLite shares no delete access), so the file
        // a rebuild would put in its place never arrives during a call.
        #[cfg(windows)]
        ("registry file replacement refused", None, false),
    ];
    let (mut actual, mut expected) = (Vec::new(), Vec::new());
    for (change, refused, moved) in rows.iter().cloned() {
        let f = Fixture::new();
        let plan = f.plan();
        let (prepared, context) = f.prepare(SshDirection::Push);
        let expectation = f.expectation(&prepared);
        let configuration = f.service.observation_configuration(&f.root, &plan).unwrap();
        // The call's first steps: the rechecks of its connection.
        f.service.recheck_ssh(&prepared, &context).unwrap();
        let repository = git2::Repository::open(&f.root).unwrap();
        let mut handle = repository
            .remote_anonymous(&prepared.endpoint.connection_url)
            .unwrap();
        let attempt = CallbackAttempt::default();
        let mut remote = AuthenticatedSshRemote::new(
            &mut handle,
            &attempt,
            &f.service,
            &prepared,
            &context,
            None,
            None,
            identity(1),
        );
        remote.require_action_expectation(&expectation).unwrap();
        let migrations = migrations_for_testing();
        match change {
            "nothing" => {}
            "key deselected" => {
                f.service.clear_shared_key_selection().unwrap();
            }
            "another key selected" => {
                let other = register(&f.service, &f.data.join("other-key"), b"another key");
                f.service.select_shared_key(other).unwrap();
            }
            "key file replaced" => fs::write(&f.key, b"changed key source").unwrap(),
            "key file removed" => fs::remove_file(&f.key).unwrap(),
            "key removed" => {
                f.service.clear_shared_key_selection().unwrap();
                assert_eq!(
                    f.service.unregister_shared_key(f.id).unwrap(),
                    UnregisterSharedKeyOutcome::Unregistered
                );
            }
            "host pin changed" => f
                .service
                .approve_host_pin_for_testing(&prepared.endpoint.authority, &identity(2))
                .unwrap(),
            #[cfg(unix)]
            "registry file replaced" => f.replace_registry(""),
            #[cfg(unix)]
            "registry file replaced, key deselected" => {
                f.replace_registry("UPDATE shared_ssh_keys SET selected = 0")
            }
            #[cfg(windows)]
            "registry file replacement refused" => assert!(
                crate::repository::replace_corrupt_registry(&f.data.join(REGISTRY_FILE), &f.root)
                    .is_err()
            ),
            other => panic!("{other}"),
        }
        // The next steps: the recheck before a listing or a transfer, and
        // the configuration the call's checks read.
        let checked = remote
            .require_action_expectation(&expectation)
            .map_err(|error| error.kind);
        let listed = remote.fresh_advertisement().map_err(|error| error.kind);
        let read = f
            .service
            .observation_configuration_in(remote.call_registry(), &f.root, &plan)
            .unwrap();
        actual.push((
            change,
            checked,
            listed.map(|_| ()),
            read != configuration,
            migrations_for_testing() - migrations,
        ));
        // A file put in place of the registry is migrated, as a fresh open
        // migrates it, once.
        let replaced = change.starts_with("registry file replaced");
        expected.push((
            change,
            refused.clone().map_or(Ok(()), Err),
            Err(refused.unwrap_or(ProtocolFailure)),
            moved,
            usize::from(replaced),
        ));
    }
    assert_eq!(actual, expected);
}

/// The call's registry connection keeps this process's lock on the
/// registry for the whole call (review C1). On Unix, closing any
/// descriptor of a file drops every POSIX lock the process holds on it,
/// SQLite's included; with its lock gone, another process's last close
/// checkpoints and deletes the WAL and its index, and the call would go on
/// reading the rows of the old ones. So no read, of the call or of a
/// connection of its own, may open and close the registry outside SQLite.
#[cfg(target_os = "linux")]
#[test]
fn a_call_keeps_its_lock_on_the_registry_across_reads_and_opens() {
    use crate::repository::coordination::sqlite_shared_lock_held;
    let f = Fixture::new();
    let registry = f.data.join(REGISTRY_FILE);
    let plan = f.plan();
    let (prepared, context) = f.prepare(SshDirection::Fetch);
    let mut held = vec![("the call's first read", sqlite_shared_lock_held(&registry))];
    f.service.recheck_ssh(&prepared, &context).unwrap();
    held.push((
        "a recheck on the call's connection",
        sqlite_shared_lock_held(&registry),
    ));
    f.service
        .observation_configuration_in(&prepared.registry, &f.root, &plan)
        .unwrap();
    held.push((
        "the call's configuration read",
        sqlite_shared_lock_held(&registry),
    ));
    f.service.list_shared_keys().unwrap();
    held.push((
        "a key read on a connection of its own",
        sqlite_shared_lock_held(&registry),
    ));
    f.service.observation_configuration(&f.root, &plan).unwrap();
    held.push((
        "a configuration read outside the call",
        sqlite_shared_lock_held(&registry),
    ));
    f.service.recheck_ssh(&prepared, &context).unwrap();
    held.push((
        "the call's next recheck",
        sqlite_shared_lock_held(&registry),
    ));
    let expected: Vec<_> = held.iter().map(|(read, _)| (*read, true)).collect();
    assert_eq!(held, expected);
}
