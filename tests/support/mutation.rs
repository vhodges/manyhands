//! Shared support for the mutation boundary tests: a clock the test sets,
//! the privacy scan of the request stores, and the golden envelopes under
//! `tests/fixtures/mutation_v1`.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};

use manyhands::{
    repository::{
        CancellationToken, Mutation, MutationCall, MutationDataDto, NoProgress, OperationFamily,
        OperationId, RelationshipWrite, RepositoryService, RequestId, ResolvedRepository,
        TicketCreateInput, TicketDraft, TicketMutationDto, TicketSaveInput,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        request_store::{Clock, JournalRow, RequestRecord, ScopeKey},
    },
    results::Envelope,
};
use rusqlite::types::ValueRef;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use super::{golden, items, schema};

/// A clock that reads what the test last set, in seconds.
pub struct TestClock(AtomicI64);

impl TestClock {
    pub fn at(seconds: i64) -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(seconds)))
    }

    pub fn set(&self, seconds: i64) {
        self.0.store(seconds, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}

/// A service over `data_directory` whose records are timed by `clock`.
pub fn service_with_clock(data_directory: &Path, clock: &Arc<TestClock>) -> RepositoryService {
    RepositoryService::open_at(data_directory)
        .unwrap()
        .with_clock_for_testing(clock.clone())
}

/// The three request tables and the three journals: every store a request
/// leaves a row in.
pub const REQUEST_STORES: [&str; 6] = [
    "request_records",
    "request_operations",
    "confirmation_records",
    "operation_records",
    "remote_operation_records",
    "key_material_operations",
];

/// Where a planted sentinel was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredSentinel {
    pub sentinel: String,
    pub table: &'static str,
    pub column: String,
}

fn bytes_contain(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The first of `sentinels` found in any column of any row of the request
/// stores, with where it is.
///
/// A scenario plants its sentinels in what no store may keep: a Markdown
/// body or source, a passphrase, key bytes, text a server sent. Every
/// column is read, whatever its type, as the bytes SQLite holds; a text
/// value that is JSON is also read as its strings, so a sentinel holding a
/// quote or a backslash is still found. A store that does not exist fails
/// the scan: it would otherwise pass by reading nothing.
pub fn find_stored_sentinel(data_directory: &Path, sentinels: &[&str]) -> Option<StoredSentinel> {
    for sentinel in sentinels {
        assert!(!sentinel.is_empty(), "an empty sentinel matches everything");
    }
    let connection = items::index(data_directory);
    for table in REQUEST_STORES {
        let columns: Vec<String> = connection
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
            .unwrap()
            .query_map([table], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(!columns.is_empty(), "{table}: the store does not exist");
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for (index, column) in columns.iter().enumerate() {
                let found = |sentinel: &str| StoredSentinel {
                    sentinel: sentinel.to_owned(),
                    table,
                    column: column.clone(),
                };
                let bytes = match row.get_ref(index).unwrap() {
                    ValueRef::Null => continue,
                    ValueRef::Integer(value) => value.to_string().into_bytes(),
                    ValueRef::Real(value) => value.to_string().into_bytes(),
                    ValueRef::Text(text) => text.to_vec(),
                    ValueRef::Blob(blob) => blob.to_vec(),
                };
                if let Some(sentinel) = sentinels
                    .iter()
                    .find(|sentinel| bytes_contain(&bytes, sentinel.as_bytes()))
                {
                    return Some(found(sentinel));
                }
                if let Ok(json) = serde_json::from_slice::<Value>(&bytes)
                    && let Some((sentinel, _)) = golden::find_sentinel(&json, sentinels)
                {
                    return Some(found(sentinel));
                }
            }
        }
    }
    None
}

/// Fails when any request store holds one of `sentinels`.
pub fn assert_request_stores_exclude(data_directory: &Path, sentinels: &[&str]) {
    if let Some(found) = find_stored_sentinel(data_directory, sentinels) {
        panic!(
            "{}.{} holds the sentinel {:?}",
            found.table, found.column, found.sentinel
        );
    }
}

/// Every golden envelope of the mutation boundary, by fixture name. A
/// fixture with no entry, or an entry with no fixture, fails
/// `tests/mutation_replay.rs`.
pub const CASES: &[&str] = &[
    "failure_request_not_found",
    "request_show_accepted",
    "request_show_finished",
];

pub fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mutation_v1")
}

/// Checks one envelope of the mutation boundary against the published
/// contract, as `golden::assert_mutation_contract` checks one under
/// `read_v1`: the redaction scan, the envelope schema, the registered
/// recovery actions, the data schema when the case names one, and the
/// golden fixture `tests/fixtures/mutation_v1/<name>.json`.
pub fn assert_contract(case: &golden::ContractCase<'_>, envelope: &impl Serialize) {
    let name = case.name;
    let value = serde_json::to_value(envelope).unwrap();
    assert!(
        CASES.contains(&name),
        "{name}: not listed in mutation::CASES"
    );
    if let Some((sentinel, location)) = golden::find_sentinel(&value, case.sentinels) {
        panic!("{name}: the envelope contains {sentinel:?} at {location}");
    }
    if let Err(problem) = schema::check_published(golden::ENVELOPE_SCHEMA, &value) {
        panic!(
            "{name}: the envelope does not match {}: {problem}",
            golden::ENVELOPE_SCHEMA
        );
    }
    golden::assert_registered_recovery(name, &value);
    let data = &value["data"];
    match case.data_schema {
        Some(data_schema) => {
            assert!(!data.is_null(), "{name}: a data schema needs data");
            if let Err(problem) = schema::check_published(data_schema, data) {
                panic!("{name}: data does not match {data_schema}: {problem}");
            }
        }
        None => assert!(data.is_null(), "{name}: data needs a data schema"),
    }

    let actual = golden::render(&value, case.placeholders);
    let path = fixture_directory().join(format!("{name}.json"));
    if golden::update_requested(
        std::env::var_os(golden::UPDATE_VARIABLE).as_deref(),
        std::env::var_os("CI").as_deref(),
    ) {
        fs::create_dir_all(fixture_directory()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{name}: cannot read {}: {error}; set {}=1 to write it",
            path.display(),
            golden::UPDATE_VARIABLE
        )
    });
    assert!(
        expected == actual.as_bytes(),
        "{name}: the envelope differs from {}\n--- expected\n{}\n--- actual\n{actual}",
        path.display(),
        String::from_utf8_lossy(&expected)
    );
}

/// A request ID for each request a scenario makes.
pub const REQUEST_1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
pub const REQUEST_2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";
pub const REQUEST_3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX3";

/// What a scenario saves. Each holds a sentinel that no request store and
/// no envelope may keep.
pub const TITLE: &str = "Saved SENTINEL-TITLE-7f3a";
pub const BODY: &str = "Saved body SENTINEL-BODY-91c2.\n";
pub const SENTINELS: [&str; 2] = ["SENTINEL-TITLE-7f3a", "SENTINEL-BODY-91c2"];

/// A ticket the fixture commits closed.
pub const TICKET_CLOSED: &str = items::TICKET_B;
/// A ticket no scenario's fixture holds: the one a create makes.
pub const TICKET_NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FCN";

/// A repository with two committed, indexed tickets, `TICKET_A` open and
/// `TICKET_CLOSED` closed, and neither with an editing context.
///
/// Every envelope `execute` returns is scanned for the sentinels, and the
/// request stores are scanned when the world is dropped, so each scenario
/// that saves content is also a privacy scenario.
pub struct World {
    pub service: RepositoryService,
    pub data: tempfile::TempDir,
    pub root: PathBuf,
    pub fixture: super::TestRepository,
}

impl World {
    pub fn new() -> Self {
        Self::with_service(|data| RepositoryService::open_at(data).unwrap())
    }

    /// The same world, with the service `open` gives for its data
    /// directory: one with a failure point, say.
    pub fn with_service(open: impl FnOnce(&Path) -> RepositoryService) -> Self {
        let fixture = super::born_repository();
        let enabled = super::enabled_repository(&fixture);
        let root = fixture.root.canonicalize().unwrap();
        let open_ticket = items::ticket_path(items::TICKET_A);
        items::write(
            &root,
            &open_ticket,
            &items::ticket_source(items::TICKET_A, "Open ticket", ""),
        );
        let closed = items::ticket_path(TICKET_CLOSED);
        items::write(
            &root,
            &closed,
            &items::ticket_source_with(
                TICKET_CLOSED,
                "Closed ticket",
                "bug",
                "done",
                items::CLOSURE,
            ),
        );
        items::commit(&fixture, &[&open_ticket, &closed], items::COMMITTED_AT);
        items::refresh_completely(&enabled.service, &root);
        let super::EnabledRepository {
            service,
            data_directory,
        } = enabled;
        drop(service);
        let service = open(data_directory.path());
        Self {
            service,
            data: data_directory,
            root,
            fixture,
        }
    }

    /// Another service over the same data, as a new process would open.
    pub fn reopen(&mut self) {
        self.service = RepositoryService::open_at(self.data.path()).unwrap();
    }

    pub fn resolved(&self) -> ResolvedRepository {
        self.service.resolve_repository(&self.root).unwrap()
    }

    /// The token `show_item` gives for the item now.
    pub fn token(&self, id: &str) -> String {
        self.service
            .show_item(&self.resolved(), &items::item_id(id))
            .unwrap()
            .observation
            .unwrap()
    }

    pub fn draft(title: &str, body: &str) -> TicketDraft {
        TicketDraft {
            title: title.to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: body.to_owned(),
        }
    }

    /// A save of `id` with the sentinel title and body and no change to
    /// its relationships.
    pub fn save(&self, id: &str, observation: &str) -> Mutation {
        Mutation::TicketSave(self.save_input(id, observation))
    }

    pub fn save_input(&self, id: &str, observation: &str) -> TicketSaveInput {
        TicketSaveInput {
            root: self.root.clone(),
            id: items::item_id(id),
            draft: Self::draft(TITLE, BODY),
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation: observation.to_owned(),
        }
    }

    /// A create of `id` with the sentinel title and body and no
    /// relationship.
    pub fn create(&self, id: &str) -> Mutation {
        Mutation::TicketCreate(self.create_input(id))
    }

    pub fn create_input(&self, id: &str) -> TicketCreateInput {
        TicketCreateInput {
            root: self.root.clone(),
            id: items::item_id(id),
            draft: Self::draft(TITLE, BODY),
            deps: Vec::new(),
            parent: None,
            initials: None,
        }
    }

    pub fn execute(&self, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        execute_with(&self.service, request_id, mutation)
    }

    pub fn record(&self, request_id: &str) -> Option<RequestRecord> {
        self.service
            .request_record(RequestId::parse(request_id).unwrap())
            .unwrap()
    }

    /// Where a local operation stands in its journal.
    pub fn journal_row(&self, operation_id: &str) -> JournalRow {
        self.service
            .journal_row(
                OperationFamily::Local,
                &ScopeKey::for_repository(&self.root).unwrap(),
                OperationId::parse(operation_id).unwrap(),
            )
            .unwrap()
    }

    /// How many rows the local journal holds that are not completed.
    pub fn pending_journal_rows(&self) -> i64 {
        items::index(self.data.path())
            .query_row(
                "SELECT COUNT(*) FROM operation_records WHERE state != 'completed'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub fn request_rows(&self) -> i64 {
        items::index(self.data.path())
            .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
            .unwrap()
    }

    pub fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    pub fn branch(id: &str) -> String {
        format!("manyhands/ticket/{id}")
    }

    /// The tip of the item's editing branch, if it has one.
    pub fn branch_tip(&self, id: &str) -> Option<git2::Oid> {
        git2::Repository::open(&self.root)
            .unwrap()
            .find_branch(&Self::branch(id), git2::BranchType::Local)
            .ok()
            .and_then(|branch| branch.get().target())
    }

    pub fn primary_head(&self) -> git2::Oid {
        git2::Repository::open(&self.root)
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap()
    }

    /// How many commits the item's editing branch has that primary does
    /// not: the checkpoints made in its context.
    pub fn branch_commits(&self, id: &str) -> usize {
        let Some(tip) = self.branch_tip(id) else {
            return 0;
        };
        let repository = git2::Repository::open(&self.root).unwrap();
        let mut walk = repository.revwalk().unwrap();
        walk.push(tip).unwrap();
        walk.hide(self.primary_head()).unwrap();
        walk.count()
    }

    /// The ticket's file in its editing context.
    pub fn worktree_source(&self, id: &str) -> String {
        fs::read_to_string(self.worktree(id).join(items::ticket_path(id))).unwrap()
    }

    /// The ticket's file in the primary worktree.
    pub fn primary_source(&self, id: &str) -> String {
        fs::read_to_string(self.root.join(items::ticket_path(id))).unwrap()
    }

    /// Asserts that nothing is left of a rejected request: no record, no
    /// journal row in flight and, for `id`, no editing context or branch.
    pub fn assert_nothing_exists(&self, request_id: &str, id: &str) {
        assert!(self.record(request_id).is_none(), "a request record");
        assert_eq!(self.pending_journal_rows(), 0, "a pending journal row");
        assert!(!self.worktree(id).exists(), "an editing context");
        assert!(self.branch_tip(id).is_none(), "an editing branch");
    }
}

impl Drop for World {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert_request_stores_exclude(self.data.path(), &SENTINELS);
            super::assert_operation_records_hold_no_content(self.data.path());
        }
    }
}

struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

/// Runs one request through the boundary and scans its envelope for the
/// sentinels.
pub fn execute_with(
    service: &RepositoryService,
    request_id: &str,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    execute_cancellable(service, request_id, mutation, &CancellationToken::new())
}

pub fn execute_cancellable(
    service: &RepositoryService,
    request_id: &str,
    mutation: Mutation,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: RequestId::parse(request_id).unwrap(),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        cancel,
    );
    let value = serde_json::to_value(&envelope).unwrap();
    if let Some((sentinel, location)) = golden::find_sentinel(&value, &SENTINELS) {
        panic!("the envelope contains {sentinel:?} at {location}");
    }
    if let Err(problem) = schema::check_published(golden::ENVELOPE_SCHEMA, &value) {
        panic!("the envelope does not match its schema: {problem}");
    }
    golden::assert_registered_recovery("execute", &value);
    envelope
}

impl World {
    /// Sets a value in the repository's own Git configuration.
    pub fn set_local_config(&self, key: &str, value: &str) {
        git2::Repository::open(&self.root)
            .unwrap()
            .config()
            .unwrap()
            .open_level(git2::ConfigLevel::Local)
            .unwrap()
            .set_str(key, value)
            .unwrap();
    }

    /// The repository's own Git configuration file, as it is on disk.
    pub fn local_config_bytes(&self) -> Vec<u8> {
        fs::read(self.root.join(".git/config")).unwrap()
    }

    /// Commits the Manyhands configuration on primary with `extra` keys,
    /// in the form the library writes a configuration in: authoring
    /// refuses a repository whose committed configuration is in another.
    pub fn commit_configuration(&self, extra: &str) {
        let config = manyhands::canonical::parse_repository_config(&format!(
            "{}{extra}",
            super::config_source()
        ))
        .unwrap();
        super::commit_tracked_configuration(
            &self.fixture,
            &manyhands::canonical::serialize_repository_config(&config).unwrap(),
        );
    }

    /// The short code the ticket's file holds in its editing context.
    pub fn slug(&self, id: &str) -> Option<String> {
        self.worktree_source(id)
            .lines()
            .find_map(|line| line.strip_prefix("slug: "))
            .map(str::to_owned)
    }

    /// A draft equal to what the fixture committed for `TICKET_A`.
    pub fn unchanged_draft() -> TicketDraft {
        Self::draft("Open ticket", "Body of Open ticket.\n")
    }
}

/// What a ticket request reported about its ticket.
pub fn ticket_data(envelope: &Envelope<MutationDataDto>) -> &TicketMutationDto {
    match envelope.data.as_ref().expect("ticket data") {
        MutationDataDto::Ticket(data) => data,
    }
}

/// The names of the recovery actions an envelope suggests.
pub fn recovery_actions(envelope: &Envelope<MutationDataDto>) -> Vec<&'static str> {
    envelope
        .recovery
        .iter()
        .map(|action| action.action.as_str())
        .collect()
}

static OWNED_PATH_HOOK_TEST_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

/// The owned-path hook is process-global: a test that installs one holds
/// this for as long as it can fire.
pub fn owned_path_hook_test_lock() -> std::sync::MutexGuard<'static, ()> {
    OWNED_PATH_HOOK_TEST_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
