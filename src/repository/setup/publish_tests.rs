use super::*;
use crate::{
    canonical::{self, CONFIG_PATH, RepositoryConfig},
    repository::{
        CommitIdentity, OperationFamily, OperationId, REGISTRY_FILE, RequestState, open_registry,
        register_setup_pending,
        request_store::{
            DigestSalt, IntentDigest, NewRequest, RequestOperation, RequestRecord, ScopeKey,
        },
        setup::{
            channel::SetupChannel,
            commit, lock,
            record::{self, SetupKind, SetupRecord},
        },
        transport::{SetupAdvertisement, SetupPush, SetupScope, SshDirection},
    },
    results::{Effects, ResultCode},
};
use git2::{IndexEntry, IndexTime, Repository, Signature};
use rusqlite::{Connection, params};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const REMOTE: &str = "origin";
const PRIMARY: &str = "main";

fn branch_ref() -> String {
    format!("refs/heads/manyhands/setup/{SETUP}")
}

fn base_ref() -> String {
    format!("refs/manyhands/setups/{SETUP}/base")
}

fn configuration(integration: &str) -> Vec<u8> {
    let mut unknown = toml::Table::new();
    unknown.insert("integration".into(), integration.into());
    canonical::serialize_repository_config(&RepositoryConfig {
        primary_branch: PRIMARY.into(),
        publication_remote: Some(REMOTE.into()),
        unknown,
    })
    .unwrap()
    .into_bytes()
}

fn signature() -> Signature<'static> {
    Signature::now("Fixture", "fixture@example.invalid").unwrap()
}

/// A commit on `parent` in `repository`, with `edits` applied to its tree
/// (`None` removes a path), written to `update_ref` when given. The working
/// tree, if any, is not touched.
fn commit_on(
    repository: &Repository,
    update_ref: Option<&str>,
    parent: Oid,
    message: &str,
    edits: &[(&str, Option<&[u8]>)],
) -> Oid {
    let parent = repository.find_commit(parent).unwrap();
    let mut index = git2::Index::new().unwrap();
    index.read_tree(&parent.tree().unwrap()).unwrap();
    for (path, content) in edits {
        match content {
            Some(content) => {
                let entry = IndexEntry {
                    ctime: IndexTime::new(0, 0),
                    mtime: IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644,
                    uid: 0,
                    gid: 0,
                    file_size: u32::try_from(content.len()).unwrap(),
                    id: repository.blob(content).unwrap(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                };
                index.add(&entry).unwrap();
            }
            None => index.remove_path(Path::new(path)).unwrap(),
        }
    }
    let tree = repository
        .find_tree(index.write_tree_to(repository).unwrap())
        .unwrap();
    repository
        .commit(
            update_ref,
            &signature(),
            &signature(),
            message,
            &tree,
            &[&parent],
        )
        .unwrap()
}

/// What a call of the stub's does at a push.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PushFault {
    /// The push lands.
    None,
    /// The call fails before the push, and nothing is sent.
    Before(SshTransportErrorKind),
    /// The push lands and the call still fails: a lost status.
    LandsThenFails(SshTransportErrorKind),
    /// The call answers that it sent the push, and nothing landed.
    Lost,
}

/// A channel over a bare repository that stands for the remote: it logs
/// each connection it is asked to open, in order, and can fail any of them.
struct Stub {
    local: PathBuf,
    remote: PathBuf,
    log: Vec<&'static str>,
    /// `(call, nth call of that name counting from 0, kind)`.
    fails: Vec<(&'static str, usize, SshTransportErrorKind)>,
    push: PushFault,
}

impl Stub {
    fn error(
        &self,
        scope: &SetupScope,
        direction: SshDirection,
        kind: SshTransportErrorKind,
    ) -> SshTransportError {
        SshTransportError {
            root: self.local.clone(),
            remote_name: scope.remote_name.clone(),
            direction,
            selected_key_id: None,
            authority: None,
            kind,
        }
    }

    fn enter(
        &mut self,
        call: &'static str,
        scope: &SetupScope,
        direction: SshDirection,
    ) -> Result<(), SshTransportError> {
        let nth = self.log.iter().filter(|logged| **logged == call).count();
        self.log.push(call);
        match self
            .fails
            .iter()
            .find(|(name, at, _)| *name == call && *at == nth)
        {
            Some((_, _, kind)) => Err(self.error(scope, direction, kind.clone())),
            None => Ok(()),
        }
    }

    fn remote(&self) -> Repository {
        Repository::open(&self.remote).unwrap()
    }
}

impl SetupChannel for Stub {
    fn fetch_base(&mut self, scope: &SetupScope) -> Result<SetupAdvertisement, SshTransportError> {
        self.enter("fetch", scope, SshDirection::Fetch)?;
        let remote = self.remote();
        let target = |name: &str| remote.find_reference(name).ok().and_then(|r| r.target());
        let found = SetupAdvertisement {
            primary: target(&format!("refs/heads/{}", scope.primary_branch)),
            setup: target(&scope.branch_ref()),
        };
        if found.primary.is_some() {
            let local = Repository::open(&self.local).unwrap();
            local
                .remote_anonymous(self.remote.to_str().unwrap())
                .unwrap()
                .fetch(
                    &[format!(
                        "+refs/heads/{}:{}",
                        scope.primary_branch,
                        scope.base_ref()
                    )],
                    None,
                    None,
                )
                .unwrap();
        }
        Ok(found)
    }

    fn push_branch(
        &mut self,
        scope: &SetupScope,
        commit: Oid,
    ) -> Result<SetupPush, SshTransportError> {
        self.enter("push", scope, SshDirection::Push)?;
        if let PushFault::Before(kind) = self.push.clone() {
            return Err(self.error(scope, SshDirection::Push, kind));
        }
        if self.push == PushFault::Lost {
            return Ok(SetupPush::Sent);
        }
        let local = Repository::open(&self.local).unwrap();
        assert_eq!(
            local.find_reference(&scope.branch_ref()).unwrap().target(),
            Some(commit),
            "the push is of the local setup branch, at the recorded commit"
        );
        local
            .remote_anonymous(self.remote.to_str().unwrap())
            .unwrap()
            .push(&[format!("{0}:{0}", scope.branch_ref())], None)
            .unwrap();
        if let PushFault::LandsThenFails(kind) = self.push.clone() {
            return Err(self.error(scope, SshDirection::Push, kind));
        }
        Ok(SetupPush::Sent)
    }

    fn observe_branch(&mut self, scope: &SetupScope) -> Result<Option<Oid>, SshTransportError> {
        self.enter("observe", scope, SshDirection::Fetch)?;
        Ok(self
            .remote()
            .find_reference(&scope.branch_ref())
            .ok()
            .and_then(|reference| reference.target()))
    }
}

struct Fixture {
    service: RepositoryService,
    data: tempfile::TempDir,
    _local_directory: tempfile::TempDir,
    remote_directory: tempfile::TempDir,
    root: PathBuf,
    kind: SetupKind,
    repository_id: i64,
    request_id: RequestId,
    operation_id: OperationId,
    /// The commit the local repository and the remote began from.
    c0: Oid,
}

impl Fixture {
    /// A registered repository on `main` with a repository-local identity,
    /// a remote that holds the same primary, and a recorded setup of `kind`
    /// made by an accepted request whose own row completed at
    /// `setup_recorded`. A change begins from a primary that holds a
    /// configuration; a first-time setup from one that holds none.
    fn new(kind: SetupKind) -> Self {
        let local_directory = tempfile::tempdir().unwrap();
        let root = local_directory.path().canonicalize().unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head(PRIMARY);
        let local = Repository::init_opts(&root, &options).unwrap();
        {
            let mut config = local.config().unwrap();
            config.set_str("user.name", "Fixture").unwrap();
            config
                .set_str("user.email", "fixture@example.invalid")
                .unwrap();
        }
        let mut index = local.index().unwrap();
        let mut files: Vec<(&str, Vec<u8>)> = vec![("README.md", b"readme".to_vec())];
        if kind == SetupKind::Change {
            files.push((CONFIG_PATH, configuration("direct")));
        }
        for (path, content) in &files {
            let full = root.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(&full, content).unwrap();
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = local.find_tree(index.write_tree().unwrap()).unwrap();
        let c0 = local
            .commit(Some("HEAD"), &signature(), &signature(), "Base", &tree, &[])
            .unwrap();
        local
            .remote(REMOTE, "ssh://git@github.com/team/repo.git")
            .unwrap();
        let base_configuration_oid = (kind == SetupKind::Change)
            .then(|| tree.get_path(Path::new(CONFIG_PATH)).unwrap().id());
        drop(tree);

        // The remote: a bare clone of the local repository.
        let remote_directory = tempfile::tempdir().unwrap();
        git2::build::RepoBuilder::new()
            .bare(true)
            .clone(root.to_str().unwrap(), remote_directory.path())
            .unwrap();

        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let request_id = RequestId::new();
        let operation_id = OperationId::new();
        let mut db = open_registry(&data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap();
        let tx = db.transaction().unwrap();
        let repository_id = register_setup_pending(&tx, &root).unwrap();
        record::insert(
            &tx,
            repository_id,
            &SetupRecord {
                setup_id: SETUP.parse().unwrap(),
                kind,
                remote_name: REMOTE.into(),
                primary_branch: PRIMARY.into(),
                endpoint_digest: [1; 32],
                configuration: configuration("pull_request"),
                base_configuration_oid,
                base_oid: None,
                commit_oid: None,
                published_oid: None,
                request_id,
                created_at: 1_700_000_000,
            },
        )
        .unwrap();
        tx.commit().unwrap();
        drop(db);

        let command = match kind {
            SetupKind::Enable => "repo enable",
            SetupKind::Change => "remote select",
        };
        let scope = ScopeKey::for_repository(&root).unwrap();
        let digest =
            IntentDigest::builder(DigestSalt::Request(request_id), command, &scope, "").finish();
        service
            .insert_request(&NewRequest {
                request_id,
                scope,
                command: command.into(),
                target: String::new(),
                intent_digest: digest,
                confirmation: None,
                base_ref: None,
                base_oid: None,
                expected_digest: None,
                operations: vec![RequestOperation {
                    family: OperationFamily::Local,
                    operation_id,
                }],
            })
            .unwrap();
        let fixture = Self {
            service,
            data,
            _local_directory: local_directory,
            remote_directory,
            root,
            kind,
            repository_id,
            request_id,
            operation_id,
            c0,
        };
        fixture
            .db()
            .execute(
                "INSERT INTO operation_records
                    (repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at)
                 VALUES (?1,?2,?3,?4,'','completed','setup_recorded',1)",
                params![
                    repository_id,
                    fixture.root.to_str().unwrap(),
                    operation_id.to_string(),
                    if kind == SetupKind::Enable { "enable" } else { "set_publication_remote" },
                ],
            )
            .unwrap();
        fixture
    }

    fn db(&self) -> Connection {
        open_registry(&self.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap()
    }

    fn git(&self) -> Repository {
        Repository::open(&self.root).unwrap()
    }

    fn remote(&self) -> Repository {
        Repository::open(self.remote_directory.path()).unwrap()
    }

    fn stub(&self) -> Stub {
        Stub {
            local: self.root.clone(),
            remote: self.remote_directory.path().to_owned(),
            log: Vec::new(),
            fails: Vec::new(),
            push: PushFault::None,
        }
    }

    fn run(&self, stub: &mut Stub) -> Result<Published, SetupError> {
        continue_setup(&self.service, &self.root, self.request_id, stub)
    }

    /// `run`, meeting a held lock as busy at once, with no real wait.
    fn run_without_waiting(&self, stub: &mut Stub) -> Result<Published, SetupError> {
        continue_setup_waiting(
            &self.service,
            &self.root,
            self.request_id,
            stub,
            Some(Duration::ZERO),
        )
    }

    fn record(&self) -> Option<SetupRecord> {
        record::read(&self.db(), self.repository_id).unwrap()
    }

    fn request(&self) -> RequestRecord {
        self.service
            .request_record(self.request_id)
            .unwrap()
            .unwrap()
    }

    fn remote_ref(&self, name: &str) -> Option<Oid> {
        self.remote()
            .find_reference(name)
            .ok()
            .and_then(|r| r.target())
    }

    fn local_ref(&self, name: &str) -> Option<Oid> {
        self.git()
            .find_reference(name)
            .ok()
            .and_then(|r| r.target())
    }

    fn registrations(&self) -> i64 {
        self.db()
            .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))
            .unwrap()
    }

    fn lock_file(&self) -> PathBuf {
        self.root.join(".git").join("manyhands-setup.lock")
    }

    /// The tip of the remote's primary.
    fn remote_primary(&self) -> Oid {
        self.remote_ref(&format!("refs/heads/{PRIMARY}")).unwrap()
    }

    fn identity() -> CommitIdentity {
        CommitIdentity {
            name: "Fixture".into(),
            email: "fixture@example.invalid".into(),
        }
    }

    /// The state of a crash after the setup commit was made and recorded
    /// on `base`, before anything else: its object, and the record.
    fn made_and_recorded(&self, base: Oid) -> Oid {
        let record = self.record().unwrap();
        let message = match self.kind {
            SetupKind::Enable => commit::ENABLE_MESSAGE,
            SetupKind::Change => commit::CHANGE_MESSAGE,
        };
        let made = commit::build(
            &self.git(),
            base,
            &record.configuration,
            message,
            &Self::identity(),
        )
        .unwrap();
        let mut db = self.db();
        let tx = db.transaction().unwrap();
        record::record_commit(&tx, self.repository_id, base, made).unwrap();
        tx.commit().unwrap();
        made
    }

    /// Ends the setup as a cancellation does: its request finished with
    /// `code`, the record gone, in one transaction.
    fn end_elsewhere(&self, code: ResultCode) {
        let mut db = self.db();
        let tx = db.transaction().unwrap();
        assert!(
            crate::repository::mutation::finish_setup_request(
                &tx,
                self.repository_id,
                self.request_id,
                code,
                None,
                1_700_000_001
            )
            .unwrap()
        );
        assert!(record::delete(&tx, self.repository_id).unwrap());
        tx.commit().unwrap();
    }

    fn operation_row(&self) -> Option<(String, String, Option<i64>)> {
        self.db()
            .query_row(
                "SELECT state, completed_step, repository_id FROM operation_records WHERE operation_ulid = ?1",
                [self.operation_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok()
    }
}

fn kinds(error: &SetupError) -> ResultCode {
    error.code()
}

/// Every reference of `repository`, by name.
fn refs(repository: &Repository) -> BTreeMap<String, Option<Oid>> {
    repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            (reference.name().unwrap().to_owned(), reference.target())
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Outcome {
    Publishes,
    Refuses(ResultCode),
}

struct Scenario {
    name: &'static str,
    kind: SetupKind,
    /// Brings the remote and the local repository to the state the case is
    /// about.
    arrange: fn(&Fixture),
    outcome: Outcome,
}

fn scenarios() -> Vec<Scenario> {
    use Outcome::{Publishes, Refuses};
    use SetupKind::{Change, Enable};
    fn nothing(_: &Fixture) {}
    vec![
        Scenario {
            name: "the remote advertises no primary",
            kind: Enable,
            arrange: |f| {
                f.remote()
                    .find_reference(&format!("refs/heads/{PRIMARY}"))
                    .unwrap()
                    .delete()
                    .unwrap();
            },
            outcome: Refuses(ResultCode::RemotePrimaryMissing),
        },
        Scenario {
            name: "the local primary is the base",
            kind: Enable,
            arrange: nothing,
            outcome: Publishes,
        },
        Scenario {
            name: "the local primary is behind the base",
            kind: Enable,
            arrange: |f| {
                commit_on(
                    &f.remote(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Remote only",
                    &[("remote.txt", Some(b"r"))],
                );
            },
            outcome: Publishes,
        },
        Scenario {
            name: "the local primary is ahead of the base",
            kind: Enable,
            arrange: |f| {
                commit_on(
                    &f.git(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Local only",
                    &[("local.txt", Some(b"l"))],
                );
            },
            outcome: Refuses(ResultCode::PrimaryHasLocalCommits),
        },
        Scenario {
            name: "the local primary and the base diverged",
            kind: Enable,
            arrange: |f| {
                commit_on(
                    &f.git(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Local only",
                    &[("local.txt", Some(b"l"))],
                );
                commit_on(
                    &f.remote(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Remote only",
                    &[("remote.txt", Some(b"r"))],
                );
            },
            outcome: Refuses(ResultCode::PrimaryHasLocalCommits),
        },
        Scenario {
            name: "a first-time setup, and the base holds a configuration",
            kind: Enable,
            arrange: |f| {
                commit_on(
                    &f.remote(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Someone set it up",
                    &[(CONFIG_PATH, Some(&configuration("direct")))],
                );
            },
            outcome: Refuses(ResultCode::RemoteAlreadySetUp),
        },
        Scenario {
            name: "a change, and the base holds the configuration it began from",
            kind: Change,
            arrange: nothing,
            outcome: Publishes,
        },
        Scenario {
            name: "a change, and the base holds another configuration",
            kind: Change,
            arrange: |f| {
                commit_on(
                    &f.remote(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Someone changed it",
                    &[(CONFIG_PATH, Some(b"another configuration"))],
                );
            },
            outcome: Refuses(ResultCode::ExternalChange),
        },
        Scenario {
            name: "a change, and the base holds no configuration",
            kind: Change,
            arrange: |f| {
                commit_on(
                    &f.remote(),
                    Some(&format!("refs/heads/{PRIMARY}")),
                    f.c0,
                    "Someone removed it",
                    &[(CONFIG_PATH, None)],
                );
            },
            outcome: Refuses(ResultCode::ExternalChange),
        },
    ]
}

#[test]
fn s1_to_s4_publish_decides_on_the_fetched_base() {
    for scenario in scenarios() {
        let name = scenario.name;
        let f = Fixture::new(scenario.kind);
        (scenario.arrange)(&f);
        let local_primary = f.local_ref(&format!("refs/heads/{PRIMARY}")).unwrap();
        let base = f.remote_ref(&format!("refs/heads/{PRIMARY}"));
        let mut stub = f.stub();
        let result = f.run(&mut stub);
        match scenario.outcome {
            Outcome::Publishes => {
                let published = result.unwrap_or_else(|error| panic!("{name}: {error:?}"));
                let base = base.unwrap();
                // S1: the setup commit is on the base, with only the
                // configuration changed.
                let git = f.git();
                let made = git.find_commit(published.commit_oid).unwrap();
                assert_eq!(made.parent_ids().collect::<Vec<_>>(), vec![base], "{name}");
                let entry = made
                    .tree()
                    .unwrap()
                    .get_path(Path::new(CONFIG_PATH))
                    .unwrap();
                assert_eq!(entry.filemode(), 0o100644, "{name}");
                assert_eq!(
                    git.find_blob(entry.id()).unwrap().content(),
                    f.record().unwrap().configuration,
                    "{name}"
                );
                // The branch is local and remote, at the commit, and the
                // record says so.
                assert_eq!(
                    f.local_ref(&branch_ref()),
                    Some(published.commit_oid),
                    "{name}"
                );
                assert_eq!(
                    f.remote_ref(&branch_ref()),
                    Some(published.commit_oid),
                    "{name}"
                );
                let record = f.record().unwrap();
                assert_eq!(
                    (record.base_oid, record.commit_oid, record.published_oid),
                    (
                        Some(base),
                        Some(published.commit_oid),
                        Some(published.commit_oid)
                    ),
                    "{name}"
                );
                assert_eq!(published.setup_id, SETUP.parse().unwrap(), "{name}");
                assert_eq!(
                    published.link.source,
                    format!("manyhands/setup/{SETUP}"),
                    "{name}"
                );
                assert_eq!(published.link.target, PRIMARY, "{name}");
                assert!(
                    published.link.url.as_deref().is_some_and(
                        |url| url.starts_with("https://github.com/team/repo/compare/main...")
                    ),
                    "{name}: {:?}",
                    published.link
                );
                // Nothing moved: the local primary stays where it was, the
                // remote's primary too, and the request is not finished
                // by publishing (its binding settles it).
                assert_eq!(
                    f.local_ref(&format!("refs/heads/{PRIMARY}")),
                    Some(local_primary),
                    "{name}"
                );
                assert_eq!(
                    f.remote_ref(&format!("refs/heads/{PRIMARY}")),
                    Some(base),
                    "{name}"
                );
                assert_eq!(f.request().state, RequestState::Accepted, "{name}");
                assert_eq!(stub.log, ["fetch", "observe", "push", "observe"], "{name}");
            }
            Outcome::Refuses(code) => {
                let error = result.err().unwrap_or_else(|| panic!("{name}: published"));
                assert_eq!(kinds(&error), code, "{name}: {error:?}");
                if let SetupError::PrimaryHasLocalCommits { commits, complete } = &error {
                    // Only the commits the base lacks, newest first.
                    let summaries: Vec<_> = commits
                        .iter()
                        .map(|(_, summary)| summary.as_str())
                        .collect();
                    assert_eq!(summaries, ["Local only"], "{name}");
                    assert_eq!(commits[0].0, local_primary, "{name}");
                    assert!(complete, "{name}");
                }
                assert_eq!(stub.log, ["fetch"], "{name}");
                assert_eq!(f.remote_ref(&branch_ref()), None, "{name}");
            }
        }
    }
}

#[test]
fn s2_to_s4_a_refused_setup_is_ended() {
    for scenario in scenarios() {
        let Outcome::Refuses(code) = scenario.outcome else {
            continue;
        };
        let name = scenario.name;
        let f = Fixture::new(scenario.kind);
        (scenario.arrange)(&f);
        let mut stub = f.stub();
        let error = f.run(&mut stub).err().unwrap();
        assert_eq!(error.code(), code, "{name}");

        // The private base ref is gone, and so is the record.
        assert_eq!(f.local_ref(&base_ref()), None, "{name}");
        assert_eq!(f.record(), None, "{name}");
        // A first-time setup takes its registration with it; a change's
        // repository carries on.
        let expected_registrations = i64::from(scenario.kind == SetupKind::Change);
        assert_eq!(f.registrations(), expected_registrations, "{name}");
        // The request is finished with the refusal, in the same
        // transaction, and holds nothing else.
        let request = f.request();
        assert_eq!(request.state, RequestState::Finished, "{name}");
        let result = request.result.unwrap();
        assert_eq!(result.code, code, "{name}");
        assert_eq!(result.effects, Effects::not_requested(), "{name}");
        // It stores what a replay answers with: the setup as it stood, or for
        // a refusal for local commits those commits.
        let data = result.data.unwrap_or_else(|| panic!("{name}: data"));
        if code == ResultCode::PrimaryHasLocalCommits {
            assert_eq!(data["type"], "primary_local_commits", "{name}");
        } else {
            assert_eq!(data["type"], "setup", "{name}");
            assert_eq!(data["state"], "recorded", "{name}");
            assert_eq!(data["request_id"], f.request_id.to_string(), "{name}");
        }
        // Its own operation row is kept, completed.
        let (state, step, repository) = f.operation_row().unwrap_or_else(|| panic!("{name}: row"));
        assert_eq!(
            (state.as_str(), step.as_str()),
            ("completed", "setup_recorded"),
            "{name}"
        );
        assert_eq!(
            repository.is_some(),
            scenario.kind == SetupKind::Change,
            "{name}: the row outlives a registration it was made for"
        );
        // The same request again is answered with what it was finished
        // with, and no connection is opened.
        let again = f.run(&mut stub).err().unwrap();
        match again {
            SetupError::Ended(stored) => assert_eq!(stored.code, code, "{name}"),
            other => panic!("{name}: {other:?}"),
        }
        assert_eq!(stub.log, ["fetch"], "{name}");
    }

    // A request that is accepted and that the guard of `finish_accepted`
    // refuses (its own row is not at `setup_recorded`) is not left accepted
    // with its record gone: the ending is rolled back and the record kept.
    let f = Fixture::new(SetupKind::Enable);
    f.remote()
        .find_reference(&format!("refs/heads/{PRIMARY}"))
        .unwrap()
        .delete()
        .unwrap();
    f.db()
        .execute(
            "UPDATE operation_records SET completed_step = 'initialization_committed'",
            [],
        )
        .unwrap();
    let mut stub = f.stub();
    let error = f.run(&mut stub).err().unwrap();
    assert_eq!(error.code(), ResultCode::RecoveryRequired);
    assert!(f.record().is_some(), "the record is kept");
    assert_eq!(f.registrations(), 1);
    assert_eq!(f.request().state, RequestState::Accepted);
}

/// A stop before the push keeps the record, leaves the request accepted
/// and is continued by the same request.
#[test]
fn s5_a_stop_before_the_push_keeps_the_record() {
    use PushFault::{Before, LandsThenFails};
    use SshTransportErrorKind as Kind;
    struct Case {
        name: &'static str,
        fails: Vec<(&'static str, usize, Kind)>,
        push: PushFault,
        no_identity: bool,
        code: ResultCode,
        log: &'static [&'static str],
        /// Whether the commit is recorded when it stops.
        made: bool,
        /// What the same request does after the stop is gone.
        then: &'static [&'static str],
    }
    let cases = [
        Case {
            name: "no selected key at the fetch",
            fails: vec![("fetch", 0, Kind::NoSelectedKey)],
            push: PushFault::None,
            no_identity: false,
            code: ResultCode::NoSelectedKey,
            log: &["fetch"],
            made: false,
            then: &["fetch", "observe", "push", "observe"],
        },
        Case {
            name: "an unlock refused at the fetch",
            fails: vec![("fetch", 0, Kind::UnlockCancelled)],
            push: PushFault::None,
            no_identity: false,
            code: ResultCode::UnlockRequired,
            log: &["fetch"],
            made: false,
            then: &["fetch", "observe", "push", "observe"],
        },
        Case {
            name: "the network down at the fetch",
            fails: vec![("fetch", 0, Kind::TransportUnavailable)],
            push: PushFault::None,
            no_identity: false,
            code: ResultCode::TransportUnavailable,
            log: &["fetch"],
            made: false,
            then: &["fetch", "observe", "push", "observe"],
        },
        Case {
            name: "no commit identity",
            fails: vec![],
            push: PushFault::None,
            no_identity: true,
            code: ResultCode::IdentityRequired,
            log: &["fetch"],
            made: false,
            then: &["fetch", "observe", "push", "observe"],
        },
        Case {
            name: "a stop at the first observation",
            fails: vec![("observe", 0, Kind::NoSelectedKey)],
            push: PushFault::None,
            no_identity: false,
            code: ResultCode::NoSelectedKey,
            log: &["fetch", "observe"],
            made: true,
            then: &["observe", "push", "observe"],
        },
        Case {
            name: "a stop at the push that is no outcome of it",
            fails: vec![],
            push: Before(Kind::NoSelectedKey),
            no_identity: false,
            code: ResultCode::NoSelectedKey,
            log: &["fetch", "observe", "push"],
            made: true,
            then: &["observe", "push", "observe"],
        },
        Case {
            name: "a push that fails and did not land",
            fails: vec![],
            push: Before(Kind::TransportUnavailable),
            no_identity: false,
            code: ResultCode::TransportUnavailable,
            log: &["fetch", "observe", "push", "observe"],
            made: true,
            then: &["observe", "push", "observe"],
        },
        Case {
            name: "a push that answered and did not land",
            fails: vec![],
            push: PushFault::Lost,
            no_identity: false,
            code: ResultCode::TransportUnavailable,
            log: &["fetch", "observe", "push", "observe"],
            made: true,
            then: &["observe", "push", "observe"],
        },
        Case {
            name: "a refused push that did not land",
            fails: vec![],
            push: Before(Kind::PushRejected),
            no_identity: false,
            code: ResultCode::PushRejected,
            log: &["fetch", "observe", "push", "observe"],
            made: true,
            then: &["observe", "push", "observe"],
        },
    ];
    let _ = LandsThenFails(Kind::PushRejected);
    for case in cases {
        let name = case.name;
        let f = Fixture::new(SetupKind::Enable);
        if case.no_identity {
            // An empty value in the repository's own configuration
            // overrides a global one.
            let mut config = f.git().config().unwrap();
            config.set_str("user.name", "").unwrap();
            config.set_str("user.email", "").unwrap();
        }
        let mut stub = f.stub();
        stub.fails = case.fails.clone();
        stub.push = case.push;
        let error = f
            .run(&mut stub)
            .err()
            .unwrap_or_else(|| panic!("{name}: published"));
        assert_eq!(error.code(), case.code, "{name}: {error:?}");
        assert_eq!(stub.log, case.log, "{name}");
        // The record is kept, with the commit recorded once there is one;
        // the request is left accepted; nothing is on the remote.
        let record = f.record().unwrap_or_else(|| panic!("{name}: record"));
        assert_eq!(record.commit_oid.is_some(), case.made, "{name}");
        assert_eq!(record.published_oid, None, "{name}");
        assert_eq!(f.local_ref(&branch_ref()).is_some(), case.made, "{name}");
        assert_eq!(f.remote_ref(&branch_ref()), None, "{name}");
        assert_eq!(f.request().state, RequestState::Accepted, "{name}");
        assert_eq!(f.registrations(), 1, "{name}");
        // The same request continues.
        if case.no_identity {
            let mut config = f.git().config().unwrap();
            config.set_str("user.name", "Fixture").unwrap();
            config
                .set_str("user.email", "fixture@example.invalid")
                .unwrap();
        }
        let mut again = f.stub();
        let published = f
            .run(&mut again)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(again.log, case.then, "{name}");
        assert_eq!(
            f.remote_ref(&branch_ref()),
            Some(published.commit_oid),
            "{name}"
        );
    }
}

#[test]
fn s6_s7_s8_publish_finishes_across_crashes_and_never_pushes_twice() {
    // S6: the commit is recorded and there is no branch. Step 1 is not
    // run again, the recorded commit is reused, the branch is made, and
    // the setup is published once.
    let f = Fixture::new(SetupKind::Enable);
    let made = f.made_and_recorded(f.c0);
    assert_eq!(f.local_ref(&branch_ref()), None);
    let mut stub = f.stub();
    let published = f.run(&mut stub).unwrap();
    assert_eq!(published.commit_oid, made);
    assert_eq!(stub.log, ["observe", "push", "observe"], "S6");
    assert_eq!(f.local_ref(&branch_ref()), Some(made));
    assert_eq!(f.remote_ref(&branch_ref()), Some(made));
    assert_eq!(f.record().unwrap().published_oid, Some(made));
    // Again: the record says published, and no connection is opened.
    let again = f.run(&mut stub).unwrap();
    assert_eq!(again.commit_oid, made);
    assert_eq!(stub.log, ["observe", "push", "observe"], "published once");

    // S7: the branch is on the remote and the record does not say so. It
    // is seen at the commit, recorded and not pushed again.
    let f = Fixture::new(SetupKind::Enable);
    let made = f.made_and_recorded(f.c0);
    f.git()
        .reference(&branch_ref(), made, false, "test")
        .unwrap();
    f.git()
        .remote_anonymous(f.remote_directory.path().to_str().unwrap())
        .unwrap()
        .push(&[format!("{0}:{0}", branch_ref())], None)
        .unwrap();
    let mut stub = f.stub();
    let published = f.run(&mut stub).unwrap();
    assert_eq!(published.commit_oid, made);
    assert_eq!(stub.log, ["observe"], "S7");
    assert_eq!(f.record().unwrap().published_oid, Some(made));
    assert_eq!(f.remote_ref(&branch_ref()), Some(made));

    // The push that lands and still fails is the same case seen from the
    // other side: the second observation finds it.
    let f = Fixture::new(SetupKind::Enable);
    let mut stub = f.stub();
    stub.push = PushFault::LandsThenFails(SshTransportErrorKind::TransportUnavailable);
    let published = f.run(&mut stub).unwrap();
    assert_eq!(stub.log, ["fetch", "observe", "push", "observe"]);
    assert_eq!(f.remote_ref(&branch_ref()), Some(published.commit_oid));
    assert_eq!(
        f.record().unwrap().published_oid,
        Some(published.commit_oid)
    );

    // S8: the branch is on the remote at another commit. Nothing is
    // pushed, the record is kept, and the answer is `recovery_required`.
    let f = Fixture::new(SetupKind::Enable);
    let made = f.made_and_recorded(f.c0);
    let other = commit_on(
        &f.remote(),
        Some(&branch_ref()),
        f.c0,
        "Somebody else's",
        &[("other.txt", Some(b"o"))],
    );
    let mut stub = f.stub();
    let error = f.run(&mut stub).err().unwrap();
    assert_eq!(error.code(), ResultCode::RecoveryRequired);
    assert_eq!(stub.log, ["observe"], "S8: nothing is pushed");
    assert_eq!(f.remote_ref(&branch_ref()), Some(other));
    let record = f.record().unwrap();
    assert_eq!(
        (record.commit_oid, record.published_oid),
        (Some(made), None)
    );
    assert_eq!(f.request().state, RequestState::Accepted);

    // The same, after a push that changed nothing there: at another
    // commit after the push is `recovery_required` too.
    let f = Fixture::new(SetupKind::Enable);
    let mut stub = f.stub();
    stub.push = PushFault::Lost;
    let other_after = commit_on(
        &f.remote(),
        None,
        f.c0,
        "Somebody else's",
        &[("other.txt", Some(b"o"))],
    );
    let error = f
        .run_with_ref_set_after_push(&mut stub, other_after)
        .err()
        .unwrap();
    assert_eq!(error.code(), ResultCode::RecoveryRequired);
    assert_eq!(f.record().unwrap().published_oid, None);
}

impl Fixture {
    /// Runs with the stub's lost push, and has the branch appear on the
    /// remote at `other` after it, as a peer pushing at that moment would.
    fn run_with_ref_set_after_push(
        &self,
        stub: &mut Stub,
        other: Oid,
    ) -> Result<Published, SetupError> {
        struct Racing<'a> {
            stub: &'a mut Stub,
            other: Oid,
        }
        impl SetupChannel for Racing<'_> {
            fn fetch_base(
                &mut self,
                scope: &SetupScope,
            ) -> Result<SetupAdvertisement, SshTransportError> {
                self.stub.fetch_base(scope)
            }
            fn push_branch(
                &mut self,
                scope: &SetupScope,
                commit: Oid,
            ) -> Result<SetupPush, SshTransportError> {
                let sent = self.stub.push_branch(scope, commit)?;
                self.stub
                    .remote()
                    .reference(&scope.branch_ref(), self.other, true, "a peer")
                    .unwrap();
                Ok(sent)
            }
            fn observe_branch(
                &mut self,
                scope: &SetupScope,
            ) -> Result<Option<Oid>, SshTransportError> {
                self.stub.observe_branch(scope)
            }
        }
        continue_setup(
            &self.service,
            &self.root,
            self.request_id,
            &mut Racing { stub, other },
        )
    }
}

#[test]
fn s9_a_second_setup_operation_is_busy() {
    let f = Fixture::new(SetupKind::Enable);
    let git = f.git();
    let held = crate::repository::coordination::setup_lock(
        &git,
        &f.root,
        crate::repository::RepositoryOperation::Enable,
    )
    .unwrap();
    let mut stub = f.stub();
    let error = f.run_without_waiting(&mut stub).err().unwrap();
    assert!(matches!(error, SetupError::Busy), "{error:?}");
    assert_eq!(error.code(), ResultCode::Busy);
    // Nothing interleaved: no connection, no write, the request accepted.
    assert!(stub.log.is_empty());
    let record = f.record().unwrap();
    assert_eq!((record.base_oid, record.commit_oid), (None, None));
    assert_eq!(f.request().state, RequestState::Accepted);
    assert_eq!(f.local_ref(&branch_ref()), None);
    // The same request continues once the lock is let go.
    drop(held);
    assert!(f.run(&mut stub).is_ok());

    // The repository lease is another lock: a writer that holds it makes
    // the commit step `busy` too, after the fetch, and keeps the record.
    let f = Fixture::new(SetupKind::Enable);
    let lease = crate::repository::coordination::repository_lease(
        &f.git(),
        &f.root,
        crate::repository::RepositoryOperation::Enable,
    )
    .unwrap();
    let mut stub = f.stub();
    let error = f.run_without_waiting(&mut stub).err().unwrap();
    assert!(matches!(error, SetupError::Busy), "{error:?}");
    assert_eq!(stub.log, ["fetch"]);
    assert_eq!(f.record().unwrap().commit_oid, None);
    drop(lease);
    assert!(f.run(&mut stub).is_ok());
}

#[test]
fn s10_publish_touches_only_the_setup_ref() {
    for kind in [SetupKind::Enable, SetupKind::Change] {
        let f = Fixture::new(kind);
        let git = f.git();
        let head = git.head().unwrap().target();
        let local_refs = refs(&git);
        let remote_refs = refs(&f.remote());
        let index = git.index().unwrap().write_tree().unwrap();
        let files: Vec<_> = ["README.md"]
            .iter()
            .map(|path| std::fs::read(f.root.join(path)).unwrap())
            .collect();
        drop(git);

        let mut stub = f.stub();
        let published = f.run(&mut stub).unwrap();

        // The remote gained the setup branch and nothing else.
        let mut expected = remote_refs.clone();
        expected.insert(branch_ref(), Some(published.commit_oid));
        assert_eq!(refs(&f.remote()), expected, "{kind:?}");
        // The local repository gained the setup branch and the private
        // base ref, and no remote-tracking ref.
        let git = f.git();
        let mut expected = local_refs.clone();
        expected.insert(branch_ref(), Some(published.commit_oid));
        expected.insert(base_ref(), Some(f.remote_primary()));
        assert_eq!(refs(&git), expected, "{kind:?}");
        // HEAD, the index and the working tree are where they were, and
        // the checkout is clean.
        assert_eq!(git.head().unwrap().target(), head, "{kind:?}");
        assert_eq!(git.head().unwrap().name(), Some("refs/heads/main"));
        assert_eq!(
            git.index().unwrap().write_tree().unwrap(),
            index,
            "{kind:?}"
        );
        let after: Vec<_> = ["README.md"]
            .iter()
            .map(|path| std::fs::read(f.root.join(path)).unwrap())
            .collect();
        assert_eq!(after, files, "{kind:?}");
        assert!(git.statuses(None).unwrap().is_empty(), "{kind:?}");
    }
}

#[test]
fn s11_a_changed_endpoint_stops_publishing() {
    use SshTransportErrorKind as Kind;
    // The conversion alone: the shared mapping is not changed, the setup's
    // answer is.
    for kind in [Kind::EndpointChanged, Kind::PublicationRemoteMissing] {
        assert_ne!(
            ssh_transport_error_code(&kind),
            ResultCode::RecoveryRequired
        );
        let error = SetupError::Transport(SshTransportError {
            root: PathBuf::new(),
            remote_name: REMOTE.into(),
            direction: SshDirection::Fetch,
            selected_key_id: None,
            authority: None,
            kind,
        });
        assert_eq!(error.code(), ResultCode::RecoveryRequired);
    }
    assert_eq!(
        SetupError::Transport(SshTransportError {
            root: PathBuf::new(),
            remote_name: REMOTE.into(),
            direction: SshDirection::Fetch,
            selected_key_id: None,
            authority: None,
            kind: Kind::KeyRejected,
        })
        .code(),
        ResultCode::KeyRejected
    );

    // At each connection the stop is the same: the record is kept, the
    // request left accepted, and nothing more is sent.
    type Fail = (&'static str, usize, Kind);
    let cases: [(&str, Fail, &[&str]); 4] = [
        (
            "at the fetch",
            ("fetch", 0, Kind::EndpointChanged),
            &["fetch"],
        ),
        (
            "a remote that is gone",
            ("fetch", 0, Kind::PublicationRemoteMissing),
            &["fetch"],
        ),
        (
            "at the observation",
            ("observe", 0, Kind::EndpointChanged),
            &["fetch", "observe"],
        ),
        (
            "at the push",
            ("push", 0, Kind::EndpointChanged),
            &["fetch", "observe", "push"],
        ),
    ];
    for (name, fail, log) in cases {
        let f = Fixture::new(SetupKind::Enable);
        let mut stub = f.stub();
        stub.fails = vec![fail];
        let error = f.run(&mut stub).err().unwrap();
        assert_eq!(error.code(), ResultCode::RecoveryRequired, "{name}");
        assert_eq!(stub.log, log, "{name}");
        assert!(f.record().is_some(), "{name}");
        assert_eq!(f.request().state, RequestState::Accepted, "{name}");
        assert_eq!(f.remote_ref(&branch_ref()), None, "{name}");
    }
}

#[test]
fn s12_a_setup_ended_elsewhere_answers_the_stored_envelope() {
    // Ended before the call: a cancel, a completion that merged the
    // configuration, a completion that found another one.
    for code in [
        ResultCode::Cancelled,
        ResultCode::AlreadyApplied,
        ResultCode::ExternalChange,
    ] {
        let f = Fixture::new(SetupKind::Enable);
        f.end_elsewhere(code);
        let mut stub = f.stub();
        let error = f.run(&mut stub).err().unwrap();
        match &error {
            SetupError::Ended(stored) => {
                assert_eq!(stored.code, code);
                assert_eq!(stored.effects, Effects::not_requested());
            }
            other => panic!("{code:?}: {other:?}"),
        }
        assert_eq!(error.code(), code);
        assert!(stub.log.is_empty(), "{code:?}: nothing was opened");
        assert!(
            !f.lock_file().exists(),
            "a repository with no record never makes the lock"
        );
    }

    // Ended between the first read and the lock: the second read, under
    // the lock, finds it gone.
    let f = Fixture::new(SetupKind::Change);
    let begun = lock::begin_with(&f.service, &f.root, None, || {
        f.end_elsewhere(ResultCode::Cancelled)
    });
    assert!(matches!(begun, Ok(lock::Begun::Gone)));
    assert!(f.lock_file().exists(), "the lock was taken, and let go");
    let mut stub = f.stub();
    let error = f.run(&mut stub).err().unwrap();
    assert!(matches!(&error, SetupError::Ended(stored) if stored.code == ResultCode::Cancelled));
    assert!(stub.log.is_empty());

    // A setup that is not this request's is none of its business: its
    // request is not finished, so there is nothing stored to answer.
    let f = Fixture::new(SetupKind::Enable);
    let mut stub = f.stub();
    let other = RequestId::new();
    let error = continue_setup(&f.service, &f.root, other, &mut stub)
        .err()
        .unwrap();
    assert_eq!(error.code(), ResultCode::RecoveryRequired);
    assert!(stub.log.is_empty());
    assert!(f.record().is_some());
    assert_eq!(f.request().state, RequestState::Accepted);
}

#[test]
fn a_lost_commit_object_is_made_again_only_while_the_remote_ref_is_absent() {
    // The commit was recorded, and its object did not reach the disk; the
    // branch file was written, and points at it.
    let arrange = |f: &Fixture, lost: Oid| {
        assert!(f.git().find_commit(lost).is_err());
        let mut db = f.db();
        let tx = db.transaction().unwrap();
        record::record_commit(&tx, f.repository_id, f.c0, lost).unwrap();
        tx.commit().unwrap();
        let path = f.root.join(".git/refs/heads/manyhands/setup");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(SETUP), format!("{lost}\n")).unwrap();
    };
    let nowhere = Oid::from_str(&"c".repeat(40)).unwrap();

    // The remote ref is absent: the commit is made again from the record,
    // recorded in place of the lost one, and published.
    let f = Fixture::new(SetupKind::Enable);
    arrange(&f, nowhere);
    let mut stub = f.stub();
    let published = f.run(&mut stub).unwrap();
    assert_ne!(published.commit_oid, nowhere);
    assert_eq!(stub.log, ["observe", "push", "observe"]);
    let record = f.record().unwrap();
    assert_eq!(
        (record.base_oid, record.commit_oid, record.published_oid),
        (
            Some(f.c0),
            Some(published.commit_oid),
            Some(published.commit_oid)
        )
    );
    let git = f.git();
    let made = git.find_commit(published.commit_oid).unwrap();
    assert_eq!(made.parent_ids().collect::<Vec<_>>(), vec![f.c0]);
    let entry = made
        .tree()
        .unwrap()
        .get_path(Path::new(CONFIG_PATH))
        .unwrap();
    assert_eq!(
        git.find_blob(entry.id()).unwrap().content(),
        record.configuration
    );
    assert_eq!(f.local_ref(&branch_ref()), Some(published.commit_oid));
    assert_eq!(f.remote_ref(&branch_ref()), Some(published.commit_oid));

    // A crash between recording the new commit and moving the branch: the
    // record holds a commit whose object is there, and the branch still
    // names the lost one. The same request moves it and publishes.
    let f = Fixture::new(SetupKind::Enable);
    let made = f.made_and_recorded(f.c0);
    let path = f.root.join(".git/refs/heads/manyhands/setup");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join(SETUP), format!("{nowhere}\n")).unwrap();
    assert!(f.git().find_commit(nowhere).is_err());
    let mut stub = f.stub();
    let published = f.run(&mut stub).unwrap();
    assert_eq!(published.commit_oid, made);
    assert_eq!(stub.log, ["observe", "push", "observe"]);
    assert_eq!(f.local_ref(&branch_ref()), Some(made));
    assert_eq!(f.remote_ref(&branch_ref()), Some(made));

    // The remote ref is present, at another commit or at the lost one: the
    // answer is `recovery_required`; nothing is made and nothing pushed.
    for (name, at_the_lost) in [("at another commit", false), ("at the lost commit", true)] {
        let f = Fixture::new(SetupKind::Enable);
        let (lost, at) = if at_the_lost {
            // The remote holds the commit; this clone never had its object.
            let held = commit_on(
                &f.remote(),
                Some(&branch_ref()),
                f.c0,
                "Pushed",
                &[("x", Some(b"x"))],
            );
            (held, held)
        } else {
            let other = commit_on(
                &f.remote(),
                Some(&branch_ref()),
                f.c0,
                "Somebody else's",
                &[("o", Some(b"o"))],
            );
            (nowhere, other)
        };
        arrange(&f, lost);
        let before = f.record().unwrap();
        let mut stub = f.stub();
        let error = f.run(&mut stub).err().unwrap();
        assert_eq!(error.code(), ResultCode::RecoveryRequired, "{name}");
        assert_eq!(stub.log, ["observe"], "{name}");
        assert_eq!(f.remote_ref(&branch_ref()), Some(at), "{name}");
        assert_eq!(f.record().unwrap(), before, "{name}");
        assert_eq!(f.request().state, RequestState::Accepted, "{name}");
    }
}
