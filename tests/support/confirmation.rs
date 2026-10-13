//! The worlds of the confirmed commands: a service timed by a clock the
//! test sets, over nothing, over a repository or over a key store, and
//! `prepare` with its envelope checked against the published contract.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use manyhands::{
    canonical,
    repository::{
        CommitIdentity, FailurePoint, KeyDeleteInput, Mutation, MutationDataDto, PreviewDto,
        RemoteRemoveInput, RemoteSelectInput, RemoveRegistrationRequest, RepoCreateInput,
        RepoEnableInput, RepoRemoveInput, RepositoryService, RequestId, RequestState, SharedKeyId,
        request_store::RequestRecord,
    },
    results::{Envelope, Outcome, ResultCode},
};

use super::{
    TestRepository, born_repository, golden, items,
    mutation::{
        self, KeyWorld, PASSPHRASE, PASSPHRASE_SENTINEL, REQUEST_3, TestClock, World,
        execute_confirmed, execute_with, key_data,
    },
    schema,
};

pub const PREVIEW_SCHEMA: &str = "preview.schema.json";

pub const REMOTE_REMOVAL_SCHEMA: &str = "remote_removal_preview.schema.json";

pub const REPOSITORY_REMOVAL_SCHEMA: &str = "repository_removal_preview.schema.json";

pub const KEY_DELETION_SCHEMA: &str = "key_deletion_preview.schema.json";

pub const OPERATION_ABANDON_SCHEMA: &str = "operation_abandon_preview.schema.json";

pub const IDENTITY_PREVIEW_SCHEMA: &str = "identity_preview.schema.json";

pub const HOST_PREVIEW_SCHEMA: &str = "host_preview.schema.json";

/// The published schema of the preview `command` gives.
pub fn preview_schema(command: &str) -> &'static str {
    match command {
        "remote remove" => REMOTE_REMOVAL_SCHEMA,
        "repo remove" => REPOSITORY_REMOVAL_SCHEMA,
        "key delete" => KEY_DELETION_SCHEMA,
        "operation abandon" => OPERATION_ABANDON_SCHEMA,
        "repo identity-set" => IDENTITY_PREVIEW_SCHEMA,
        "host approve" | "host replace" => HOST_PREVIEW_SCHEMA,
        "repo create" | "repo enable" => PREVIEW_SCHEMA,
        // `prepare` gives no preview of any other command.
        other => panic!("{other}: no preview schema"),
    }
}

/// `prepare`, with its envelope checked against the published contract.
pub fn prepare_with(service: &RepositoryService, mutation: &Mutation) -> Envelope<PreviewDto> {
    let envelope = service.prepare(mutation);
    let value = serde_json::to_value(&envelope).unwrap();
    schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
    golden::assert_registered_recovery("prepare", &value);
    assert_eq!(envelope.command, mutation.command());
    // A preview is of no request.
    assert_eq!(envelope.request_id, None);
    assert_eq!(envelope.operation_id, None);
    if !value["data"].is_null() {
        schema::check_published(preview_schema(mutation.command()), &value["data"]).unwrap();
    }
    envelope
}

/// When the scenarios begin.
pub const START: i64 = 1_700_000_000;

/// The message of the commit enablement makes.
pub const INITIALIZATION: &str = "Initialize Manyhands";

/// Every file and directory under `directory`, by its path below it: a
/// file with its bytes, a directory with none.
pub fn tree(directory: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, directory: &Path, found: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() && !path.is_symlink() {
                found.insert(relative, None);
                walk(root, &path, found);
            } else {
                found.insert(relative, Some(fs::read(&path).unwrap()));
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(directory, directory, &mut found);
    found
}

/// Git's global configuration file, as it is on disk, when there is one.
pub fn global_configuration() -> Option<Vec<u8>> {
    git2::Config::find_global()
        .ok()
        .and_then(|path| fs::read(path).ok())
}

/// What a preview must leave as it found it: the files under `parent`,
/// the repository list and Git's global configuration.
#[derive(Debug, PartialEq)]
pub struct Surroundings {
    pub files: BTreeMap<PathBuf, Option<Vec<u8>>>,
    pub repositories: String,
    pub global: Option<Vec<u8>>,
}

/// A service over a data directory of its own, timed by a clock the test
/// sets, with nothing registered.
pub struct Desk {
    pub service: RepositoryService,
    pub data: tempfile::TempDir,
    pub clock: Arc<TestClock>,
}

impl Desk {
    pub fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let clock = TestClock::at(START);
        let service = mutation::service_with_clock(data.path(), &clock);
        Self {
            service,
            data,
            clock,
        }
    }

    /// Another service over the same data, as a new process would open.
    pub fn reopen(&mut self) {
        self.service = mutation::service_with_clock(self.data.path(), &self.clock);
    }

    /// Another service over the same data that fails once at `point`.
    pub fn fail_once_at(&mut self, point: FailurePoint) {
        self.service =
            RepositoryService::open_at_with_failure_point_for_testing(self.data.path(), point)
                .unwrap()
                .with_clock_for_testing(self.clock.clone());
    }

    pub fn surroundings(&self, parent: &Path) -> Surroundings {
        Surroundings {
            files: tree(parent),
            repositories: serde_json::to_string(&self.service.list_repositories().unwrap())
                .unwrap(),
            global: global_configuration(),
        }
    }

    /// `prepare`, with its envelope checked against the published
    /// contract.
    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        prepare_with(&self.service, mutation)
    }

    /// A preview that is given, and the confirmation it gives.
    pub fn preview(&self, mutation: &Mutation) -> (String, PreviewDto) {
        let envelope = self.prepare(mutation);
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    pub fn execute(
        &self,
        request_id: &str,
        confirmation: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        execute_confirmed(&self.service, request_id, confirmation, mutation)
    }

    pub fn record(&self, request_id: &str) -> Option<RequestRecord> {
        self.service
            .request_record(RequestId::parse(request_id).unwrap())
            .unwrap()
    }

    pub fn count(&self, query: &str) -> i64 {
        items::index(self.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    pub fn request_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM request_records")
    }

    pub fn confirmation_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM confirmation_records")
    }

    /// The confirmations a request has accepted.
    pub fn accepted_confirmations(&self) -> i64 {
        self.count(
            "SELECT COUNT(*) FROM confirmation_records WHERE accepted_by_request IS NOT NULL",
        )
    }

    pub fn pending_journal_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'")
    }

    /// Nothing is left of a request that was rejected: no record, no
    /// confirmation held, and no operation in flight.
    pub fn assert_nothing_left(&self, request_id: &str) {
        assert_eq!(self.record(request_id), None, "a request record");
        assert_eq!(self.request_rows(), 0, "request records");
        assert_eq!(self.accepted_confirmations(), 0, "a confirmation held");
        assert_eq!(self.pending_journal_rows(), 0, "an operation in flight");
    }

    pub fn registered(&self, root: &Path) -> bool {
        self.service
            .list_repositories()
            .unwrap()
            .items
            .iter()
            .any(|repository| Path::new(&repository.root) == root)
    }
}

pub fn identity() -> CommitIdentity {
    CommitIdentity {
        name: "Ada Example".to_owned(),
        email: "ada@example.invalid".to_owned(),
    }
}

/// A Git repository with one commit on `main`, a local identity, and no
/// Manyhands configuration.
pub struct Plain {
    pub fixture: TestRepository,
    pub root: PathBuf,
}

impl Plain {
    pub fn new() -> Self {
        let fixture = born_repository();
        let root = fixture.root.canonicalize().unwrap();
        Self { fixture, root }
    }

    pub fn input(&self) -> RepoEnableInput {
        RepoEnableInput {
            root: self.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: None,
        }
    }

    pub fn enable(&self) -> Mutation {
        Mutation::RepoEnable(self.input())
    }

    pub fn head(&self) -> git2::Oid {
        self.fixture
            .repository
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
    }

    /// The commits on the checked-out branch that enablement made.
    pub fn initializations(&self) -> usize {
        initializations(&self.root)
    }

    pub fn configuration(&self) -> Option<String> {
        fs::read_to_string(self.root.join(canonical::CONFIG_PATH)).ok()
    }

    /// Checks out a new branch at the head: the worktree stays clean.
    pub fn switch_to(&self, branch: &str) {
        let head = self
            .fixture
            .repository
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap();
        if self
            .fixture
            .repository
            .find_branch(branch, git2::BranchType::Local)
            .is_err()
        {
            self.fixture
                .repository
                .branch(branch, &head, false)
                .unwrap();
        }
        self.fixture
            .repository
            .set_head(&format!("refs/heads/{branch}"))
            .unwrap();
    }

    pub fn local_config(&self, key: &str) -> Option<String> {
        local_config(&self.root, key)
    }
}

/// The value the repository's own configuration file holds for `key`.
pub fn local_config(root: &Path, key: &str) -> Option<String> {
    git2::Config::open(&root.join(".git/config"))
        .unwrap()
        .get_string(key)
        .ok()
}

pub fn initializations(root: &Path) -> usize {
    let repository = git2::Repository::open(root).unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.filter(|commit| {
        repository
            .find_commit(*commit.as_ref().unwrap())
            .unwrap()
            .message()
            == Some(INITIALIZATION)
    })
    .count()
}

/// A parent directory and a root below it that does not exist.
pub struct Site {
    pub parent: tempfile::TempDir,
    /// The root as the scope key holds it: the canonical parent joined
    /// with the name.
    pub root: PathBuf,
}

impl Site {
    pub fn new() -> Self {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().canonicalize().unwrap().join("new");
        Self { parent, root }
    }

    pub fn input(&self) -> RepoCreateInput {
        RepoCreateInput {
            // As a caller gives it, which need not be canonical.
            root: self.parent.path().join("new"),
            primary_branch: "main".to_owned(),
            identity: Some(identity()),
            slug_prefix: None,
        }
    }

    pub fn create(&self) -> Mutation {
        Mutation::RepoCreate(self.input())
    }

    pub fn head(&self) -> git2::Oid {
        git2::Repository::open(&self.root)
            .unwrap()
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
    }

    pub fn configuration(&self) -> Option<String> {
        fs::read_to_string(self.root.join(canonical::CONFIG_PATH)).ok()
    }
}

/// An enabled, registered repository with committed, indexed items, and
/// a service over it that is timed by a clock the test sets.
pub struct Home {
    pub world: World,
    pub clock: Arc<TestClock>,
}

impl Home {
    pub fn new() -> Self {
        let clock = TestClock::at(START);
        let world = World::with_service(|data| mutation::service_with_clock(data, &clock));
        Self { world, clock }
    }

    /// Another service over the same data, as a new process would open.
    pub fn reopen(&mut self) {
        self.world.service = mutation::service_with_clock(self.world.data.path(), &self.clock);
    }

    /// Another service over the same data that fails once at `point`.
    pub fn fail_once_at(&mut self, point: FailurePoint) {
        self.world.service = RepositoryService::open_at_with_failure_point_for_testing(
            self.world.data.path(),
            point,
        )
        .unwrap()
        .with_clock_for_testing(self.clock.clone());
    }

    pub fn service(&self) -> &RepositoryService {
        &self.world.service
    }

    pub fn root(&self) -> &Path {
        &self.world.root
    }

    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        prepare_with(self.service(), mutation)
    }

    /// A preview that is given, and the confirmation it gives.
    pub fn preview(&self, mutation: &Mutation) -> (String, PreviewDto) {
        let envelope = self.prepare(mutation);
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    pub fn execute(
        &self,
        request_id: &str,
        confirmation: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        execute_confirmed(self.service(), request_id, confirmation, mutation)
    }

    /// The same request again, presenting nothing: its record holds its
    /// consent.
    pub fn retry(&self, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        execute_with(self.service(), request_id, mutation)
    }

    pub fn state(&self, request_id: &str) -> Option<RequestState> {
        self.world.record(request_id).map(|record| record.state)
    }

    pub fn count(&self, query: &str) -> i64 {
        items::index(self.world.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    pub fn confirmation_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM confirmation_records")
    }

    /// Nothing is left of a request that was rejected: no record, no
    /// confirmation held, and no operation in flight.
    pub fn assert_nothing_left(&self, request_id: &str) {
        assert_eq!(self.world.record(request_id), None, "a request record");
        assert_eq!(
            self.count(
                "SELECT COUNT(*) FROM confirmation_records WHERE accepted_by_request IS NOT NULL"
            ),
            0,
            "a confirmation held"
        );
        assert_eq!(
            self.world.pending_journal_rows(),
            0,
            "an operation in flight"
        );
    }

    pub fn git(&self) -> git2::Repository {
        git2::Repository::open(self.root()).unwrap()
    }

    /// Adds a remote by hand, as `git remote add` would.
    pub fn add_remote(&self, name: &str, url: &str) {
        self.git().remote(name, url).unwrap();
    }

    pub fn remote_url(&self, name: &str) -> Option<String> {
        self.git()
            .find_remote(name)
            .ok()
            .and_then(|remote| remote.url().map(str::to_owned))
    }

    pub fn remove_remote(&self, name: &str) -> Mutation {
        Mutation::RemoteRemove(RemoteRemoveInput {
            root: self.root().to_owned(),
            name: name.to_owned(),
        })
    }

    /// Selects the publication remote, or clears the selection, as a
    /// request of its own would.
    pub fn select(&self, request_id: &str, name: Option<&str>) {
        select_through(self.service(), request_id, self.root(), name);
    }
}

impl Home {
    pub fn remove_repository(&self) -> Mutation {
        Mutation::RepoRemove(RepoRemoveInput {
            root: self.root().to_owned(),
        })
    }

    pub fn registered(&self) -> bool {
        self.service()
            .list_repositories()
            .unwrap()
            .items
            .iter()
            .any(|repository| Path::new(&repository.root) == self.root())
    }

    /// Removes the registration as another process would: through the
    /// domain operation, under an operation of its own.
    pub fn remove_registration_by_hand(&self) {
        self.service()
            .remove_registration(RemoveRegistrationRequest {
                root: self.root().to_owned(),
                operation_id: manyhands::repository::OperationId::new(),
            })
            .unwrap();
    }
}

impl Home {
    /// Registers the root again, as another process would: through the
    /// domain operation, which finds the repository enabled. The new
    /// registration is made a minute after the old one, since the time
    /// of a registration is kept in seconds.
    pub fn register_again(&self) {
        self.service()
            .enable(manyhands::repository::EnableRepositoryRequest {
                root: self.root().to_owned(),
                primary_branch: "main".to_owned(),
                identity: None,
                operation_id: manyhands::repository::OperationId::new(),
            })
            .unwrap();
        assert!(self.registered());
        let moved = items::index(self.world.data.path())
            .execute("UPDATE repositories SET enabled_at = enabled_at + 60", [])
            .unwrap();
        assert_eq!(moved, 1);
    }
}

pub fn select_through(
    service: &RepositoryService,
    request_id: &str,
    root: &Path,
    name: Option<&str>,
) {
    let selected = execute_with(
        service,
        request_id,
        Mutation::RemoteSelect(RemoteSelectInput {
            root: root.to_owned(),
            name: name.map(str::to_owned),
        }),
    );
    assert_eq!(selected.outcome, Outcome::Success, "{}", selected.message);
}

/// A service with no repository, timed by a clock the test sets, and a
/// home whose key store holds one key Manyhands generated, protected by
/// a passphrase that nothing may keep.
pub struct Keyring {
    pub world: KeyWorld,
    pub clock: Arc<TestClock>,
    /// The generated key.
    pub id: SharedKeyId,
}

impl Keyring {
    pub fn new() -> Self {
        let clock = TestClock::at(START);
        let world = KeyWorld::with_service(|data| mutation::service_with_clock(data, &clock));
        let generated = world.execute(REQUEST_3, world.generate("Work key", Some(PASSPHRASE)));
        assert_eq!(generated.code, ResultCode::Ok, "{}", generated.message);
        let id = SharedKeyId::parse(key_data(&generated).id.as_deref().unwrap()).unwrap();
        Self { world, clock, id }
    }

    /// Another service over the same data, as a new process would open.
    pub fn reopen(&mut self) {
        self.world.service = mutation::service_with_clock(self.world.data.path(), &self.clock);
    }

    /// Another service over the same data that fails once at `point`.
    pub fn fail_once_at(&mut self, point: FailurePoint) {
        self.world.service = RepositoryService::open_at_with_failure_point_for_testing(
            self.world.data.path(),
            point,
        )
        .unwrap()
        .with_clock_for_testing(self.clock.clone());
    }

    pub fn service(&self) -> &RepositoryService {
        &self.world.service
    }

    pub fn delete(&self, id: SharedKeyId) -> Mutation {
        Mutation::KeyDelete(KeyDeleteInput {
            store: self.world.store(),
            id,
        })
    }

    pub fn delete_key(&self) -> Mutation {
        self.delete(self.id)
    }

    /// `prepare`, with its envelope scanned for the passphrase.
    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        let envelope = prepare_with(self.service(), mutation);
        assert_eq!(
            golden::find_sentinel(
                &serde_json::to_value(&envelope).unwrap(),
                &[PASSPHRASE_SENTINEL]
            ),
            None
        );
        envelope
    }

    pub fn preview(&self) -> (String, PreviewDto) {
        let envelope = self.prepare(&self.delete_key());
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    pub fn scanned(&self, envelope: Envelope<MutationDataDto>) -> Envelope<MutationDataDto> {
        assert_eq!(
            golden::find_sentinel(
                &serde_json::to_value(&envelope).unwrap(),
                &[PASSPHRASE_SENTINEL]
            ),
            None
        );
        // A key command names no repository, item, branch or worktree.
        assert_eq!(envelope.scope, manyhands::results::Scope::default());
        envelope
    }

    pub fn execute(
        &self,
        request_id: &str,
        confirmation: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        self.scanned(execute_confirmed(
            self.service(),
            request_id,
            confirmation,
            mutation,
        ))
    }

    /// The same request again, presenting nothing: its record holds its
    /// consent.
    pub fn retry(&self, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        self.scanned(execute_with(self.service(), request_id, mutation))
    }

    pub fn state(&self, request_id: &str) -> Option<RequestState> {
        self.world.record(request_id).map(|record| record.state)
    }

    pub fn count(&self, query: &str) -> i64 {
        items::index(self.world.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    /// The key's two files, as its registration names them.
    pub fn files(&self) -> (PathBuf, PathBuf) {
        let directory = self.world.store_directory();
        (
            directory.join(self.id.to_string()),
            directory.join(format!("{}.pub", self.id)),
        )
    }

    pub fn registered(&self) -> bool {
        self.world.keys().iter().any(|(id, _)| *id == self.id)
    }

    /// Nothing is left of a request that was rejected: no record, no
    /// confirmation held, and no deletion begun.
    pub fn assert_nothing_left(&self, request_id: &str) {
        assert_eq!(self.world.record(request_id), None, "a request record");
        assert_eq!(
            self.count(
                "SELECT COUNT(*) FROM confirmation_records WHERE accepted_by_request IS NOT NULL"
            ),
            0,
            "a confirmation held"
        );
        assert_eq!(
            self.count("SELECT COUNT(*) FROM key_material_operations WHERE action = 'delete'"),
            0,
            "a deletion begun"
        );
    }
}

/// A repository that is not enabled, a service with nothing registered
/// and a clock that stands still, so that a preview's expiry is fixed.
pub struct Unregistered {
    pub fixture: TestRepository,
    pub root: PathBuf,
    pub data: tempfile::TempDir,
    pub service: RepositoryService,
}

impl Unregistered {
    pub fn new() -> Self {
        let fixture = born_repository();
        let root = fixture.root.canonicalize().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = Self::open(data.path(), None);
        Self {
            fixture,
            root,
            data,
            service,
        }
    }

    pub fn open(data: &Path, point: Option<FailurePoint>) -> RepositoryService {
        let clock = mutation::TestClock::at(super::operations::STORED_AT);
        match point {
            Some(point) => super::FailOnce::at(point).open_service(data),
            None => RepositoryService::open_at(data).unwrap(),
        }
        .with_clock_for_testing(clock)
    }

    pub fn fail_once_at(&mut self, point: FailurePoint) {
        self.service = Self::open(self.data.path(), Some(point));
    }

    pub fn enable(&self) -> Mutation {
        Mutation::RepoEnable(RepoEnableInput {
            root: self.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: Some("mh".to_owned()),
        })
    }
}
