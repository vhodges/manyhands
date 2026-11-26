//! A remote call reads the shared keys through one registry connection
//! (design decision 36), and each of its reads still reads the current
//! rows. No network: the call's steps run on a handle that never connected,
//! which refuses each listing and transfer after its rechecks.
use super::*;
use crate::repository::{
    REGISTRY_FILE, RequestId,
    discovery::migrations_for_testing,
    key_reads, open_registry,
    remote::{RemoteRefPlan, SynchronizationTarget, observation::endpoint_digest_of},
    setup::record::{self, SetupKind, SetupRecord},
    transport::callbacks::CallbackAttempt,
};
use std::{fs, path::PathBuf};

const CONFIGURATION: &str =
    "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
const URL: &str = "ssh://fixture@example.invalid/repository.git";
const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER_SETUP: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
const SETUP_URL: &str = "ssh://fixture@setup.example.invalid/fork.git";

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

    /// A pending setup of the remote `upstream`, which no configuration
    /// names: the file is gone, the remote is added, and the registry holds
    /// the repository and the setup's record. Its scope is the record's.
    fn pending_setup(&self) -> SetupScope {
        fs::remove_file(self.root.join(".manyhands/config.toml")).unwrap();
        let repository = git2::Repository::open(&self.root).unwrap();
        repository.remote("upstream", SETUP_URL).unwrap();
        let record = SetupRecord {
            setup_id: SETUP.parse().unwrap(),
            kind: SetupKind::Enable,
            remote_name: "upstream".into(),
            primary_branch: "main".into(),
            endpoint_digest: endpoint_digest_of(&repository, "upstream"),
            configuration: b"format_version = 2\nprimary_branch = \"main\"\n".to_vec(),
            base_configuration_oid: None,
            base_oid: None,
            commit_oid: None,
            published_oid: None,
            request_id: RequestId::new(),
            created_at: 1_700_000_000,
        };
        let mut db = open_registry(&self.data.join(REGISTRY_FILE), &mut |_| {}).unwrap();
        let root = fs::canonicalize(&self.root).unwrap();
        db.execute(
            "INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required)
             VALUES(1,?1,123,'accessible',0)",
            [root.to_str().unwrap()],
        )
        .unwrap();
        let tx = db.transaction().unwrap();
        record::insert(&tx, 1, &record).unwrap();
        tx.commit().unwrap();
        SetupScope::of(&record)
    }

    /// Replaces the setup's record with what `edit` makes of it.
    fn edit_setup(&self, edit: impl FnOnce(&mut SetupRecord)) {
        let mut db = open_registry(&self.data.join(REGISTRY_FILE), &mut |_| {}).unwrap();
        let tx = db.transaction().unwrap();
        let mut record = record::read(&tx, 1).unwrap().unwrap();
        record::delete(&tx, 1).unwrap();
        edit(&mut record);
        record::insert(&tx, 1, &record).unwrap();
        tx.commit().unwrap();
    }

    /// Removes the setup's record, as a cancel or a completion does.
    fn remove_setup(&self) {
        let mut db = open_registry(&self.data.join(REGISTRY_FILE), &mut |_| {}).unwrap();
        let tx = db.transaction().unwrap();
        assert!(record::delete(&tx, 1).unwrap());
        tx.commit().unwrap();
    }

    /// What `with_setup_remote` prepares for `scope`.
    fn prepare_setup(
        &self,
        direction: SshDirection,
        scope: &SetupScope,
    ) -> Result<(PreparedSshAttempt, SshTransportError), SshTransportErrorKind> {
        let mut context = SshTransportError {
            root: self.root.clone(),
            remote_name: String::new(),
            direction,
            selected_key_id: None,
            authority: None,
            kind: SshTransportErrorKind::ConfigurationInvalid,
        };
        self.service
            .prepare_ssh_from(
                VerifySshTransportRequest {
                    root: self.root.clone(),
                    direction,
                    approval: None,
                },
                &mut context,
                RemoteSource::Setup(scope.clone()),
            )
            .map(|prepared| (prepared, context))
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

/// A call made for a pending setup takes its remote from the scope, and each
/// recheck, the preparation's included, holds it to the record and to the
/// remote's endpoints: the record gone, another setup, another remote or
/// primary, or a remote that is not where it was is `EndpointChanged`.
#[test]
fn the_setup_scope_is_rechecked_against_the_record_and_the_endpoints() {
    use SshTransportErrorKind::EndpointChanged;
    // The digest the record holds is today's, as the observation of the
    // configured remote makes it.
    {
        let f = Fixture::new();
        let configuration = f
            .service
            .observation_configuration(&f.root, &f.plan())
            .unwrap();
        let repository = git2::Repository::open(&f.root).unwrap();
        assert_eq!(
            endpoint_digest_of(&repository, "origin"),
            configuration.endpoint_digest()
        );
    }
    // What a change made after the call's preparation does to its rechecks.
    type Change = fn(&Fixture);
    let rows: &[(&str, Change, Option<SshTransportErrorKind>)] = &[
        ("nothing", |_| {}, None),
        (
            "the record removed",
            Fixture::remove_setup,
            Some(EndpointChanged),
        ),
        (
            "another setup",
            |f| f.edit_setup(|record| record.setup_id = OTHER_SETUP.parse().unwrap()),
            Some(EndpointChanged),
        ),
        (
            "another remote",
            |f| f.edit_setup(|record| record.remote_name = "origin".into()),
            Some(EndpointChanged),
        ),
        (
            "another primary",
            |f| f.edit_setup(|record| record.primary_branch = "trunk".into()),
            Some(EndpointChanged),
        ),
        (
            "the remote's url changed",
            |f| {
                git2::Repository::open(&f.root)
                    .unwrap()
                    .remote_set_url("upstream", "ssh://fixture@moved.example.invalid/fork.git")
                    .unwrap()
            },
            Some(EndpointChanged),
        ),
        (
            // The fetch endpoint the call connects to is the same; only the
            // digest of both endpoints tells.
            "a push url added",
            |f| {
                git2::Repository::open(&f.root)
                    .unwrap()
                    .remote_set_pushurl(
                        "upstream",
                        Some("ssh://fixture@push.example.invalid/fork.git"),
                    )
                    .unwrap()
            },
            Some(EndpointChanged),
        ),
        (
            "the remote removed",
            |f| {
                git2::Repository::open(&f.root)
                    .unwrap()
                    .remote_delete("upstream")
                    .unwrap()
            },
            Some(EndpointChanged),
        ),
    ];
    let (mut actual, mut expected) = (Vec::new(), Vec::new());
    for (change, apply, refused) in rows {
        let f = Fixture::new();
        let scope = f.pending_setup();
        // The file is gone: the remote is the scope's, and the call is
        // prepared without it.
        let (prepared, context) = f.prepare_setup(SshDirection::Fetch, &scope).unwrap();
        assert_eq!(context.remote_name, "upstream");
        f.service.recheck_ssh(&prepared, &context).unwrap();
        apply(&f);
        actual.push((*change, f.service.recheck_ssh(&prepared, &context)));
        expected.push((*change, refused.clone().map_or(Ok(()), Err)));
    }
    assert_eq!(actual, expected);
    // The preparation holds the scope to the same things.
    let f = Fixture::new();
    let scope = f.pending_setup();
    let stale = SetupScope {
        endpoint_digest: [0; 32],
        ..scope.clone()
    };
    assert_eq!(
        f.prepare_setup(SshDirection::Fetch, &stale).err(),
        Some(EndpointChanged)
    );
    let other = SetupScope {
        setup_id: OTHER_SETUP.parse().unwrap(),
        ..scope.clone()
    };
    assert_eq!(
        f.prepare_setup(SshDirection::Fetch, &other).err(),
        Some(EndpointChanged)
    );
    f.edit_setup(|record| record.primary_branch = "trunk".into());
    assert_eq!(
        f.prepare_setup(SshDirection::Fetch, &scope).err(),
        Some(EndpointChanged)
    );
}

/// The four methods of the setup channel are the only ones a setup
/// connection carries, and each runs only on a setup connection in its own
/// direction: a refusal precedes any read, and a connection that never
/// connected is refused only after the rechecks of an allowed step.
#[test]
fn a_setup_connection_carries_only_the_setup_channels_methods() {
    use SshTransportErrorKind::{ConfigurationInvalid, ProtocolFailure};
    let oid = git2::Oid::from_str(&"a".repeat(40)).unwrap();
    let target = SynchronizationTarget::Context {
        kind: crate::repository::AuthoringKind::Ticket,
        item_id: SETUP.parse().unwrap(),
    };
    let kind = |result: Result<(), SshTransportError>| result.map_err(|error| error.kind);
    let (mut actual, mut expected) = (Vec::new(), Vec::new());
    for source in ["configured", "setup"] {
        for direction in [
            SshDirection::Fetch,
            SshDirection::Push,
            SshDirection::Delete,
        ] {
            let f = Fixture::new();
            let plan = f.plan();
            let (prepared, context) = if source == "setup" {
                let scope = f.pending_setup();
                f.prepare_setup(direction, &scope).unwrap()
            } else {
                f.prepare(direction)
            };
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
            let setup = source == "setup";
            let own = |wanted: SshDirection| {
                Err(if setup && direction == wanted {
                    ProtocolFailure
                } else {
                    ConfigurationInvalid
                })
            };
            let name = |method: &'static str| (source, direction, method);
            actual.push((
                name("fetch_setup_base"),
                kind(remote.fetch_setup_base().map(drop)),
            ));
            expected.push((name("fetch_setup_base"), own(SshDirection::Fetch)));
            actual.push((
                name("observe_setup_branch"),
                kind(remote.observe_setup_branch().map(drop)),
            ));
            expected.push((name("observe_setup_branch"), own(SshDirection::Fetch)));
            actual.push((
                name("push_setup_branch"),
                kind(remote.push_setup_branch(oid).map(drop)),
            ));
            expected.push((name("push_setup_branch"), own(SshDirection::Push)));
            actual.push((
                name("delete_setup_branch"),
                kind(remote.delete_setup_branch(oid).map(drop)),
            ));
            expected.push((name("delete_setup_branch"), own(SshDirection::Delete)));
            // No item method accepts a setup connection.
            if setup {
                for (method, result) in [
                    ("fetch_exact", kind(remote.fetch_exact(&plan, &target))),
                    ("push_exact", kind(remote.push_exact(&plan, &target))),
                    (
                        "delete_exact",
                        kind(remote.delete_exact(&plan, &target, oid).map(drop)),
                    ),
                    (
                        "fetch_poll_set",
                        kind(remote.fetch_poll_set(&plan, |_| true).map(drop)),
                    ),
                ] {
                    actual.push((name(method), result));
                    expected.push((name(method), Err(ConfigurationInvalid)));
                }
            }
        }
    }
    assert_eq!(actual, expected);
}
