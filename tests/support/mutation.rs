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
        CancellationToken, ConfirmationId, DocumentCreateInput, DocumentDraft, DocumentMoveInput,
        DocumentMutationDto, DocumentSaveInput, IdentityMutationDto, KeyGenerateInput,
        KeyMutationDto, Mutation, MutationCall, MutationDataDto, NoProgress, OperationFamily,
        OperationId, OperationMutationDto, PrepareDataDto, PreviewDto, ProgressEvent, ProgressSink,
        RegisterSharedKeyOutcome, RegisterSharedKeyRequest, RelationshipWrite, RemoteMutationDto,
        RepositoryService, RequestId, ResolvedRepository, SharedKeyId, SharedKeyOwnership,
        TicketCreateInput, TicketDraft, TicketMutationDto, TicketSaveInput, TicketSlugAssignInput,
        keys::{
            KeyProtection, KeyStore, PassphraseResponse, SecretPassphrase,
            SessionCredentialProvider, SessionCredentials, UnlockRequest,
        },
        request_store::{Clock, JournalRow, RequestRecord, ScopeKey},
    },
    results::Envelope,
};
use rusqlite::types::ValueRef;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use super::{golden, items, schema};
// Not every target uses every one of them.
#[allow(unused_imports)]
pub use super::mutation_golden::{
    CASES, PROGRESS_CASES, assert_contract, assert_contract_at, fixture_directory,
    progress_fixture, progress_fixture_directory,
};

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

/// The three request tables, the three journals, the recorded key
/// registry changes and the bindings of comment submissions: every store
/// a request, or a submission made through the service, leaves a row in.
pub const REQUEST_STORES: [&str; 8] = [
    "request_records",
    "request_operations",
    "confirmation_records",
    "operation_records",
    "remote_operation_records",
    "key_material_operations",
    "key_registry_changes",
    "comment_publication_bindings",
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

/// The document the fixture commits, and where.
pub const DOCUMENT: &str = items::DOCUMENT_A;
pub const DOCUMENT_PATH: &str = "docs/guide.md";
/// A document no scenario's fixture holds, and where a create puts it.
pub const DOCUMENT_NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDN";
pub const DOCUMENT_NEW_PATH: &str = "docs/notes/new.md";
/// Where a move takes `DOCUMENT`.
pub const DOCUMENT_MOVED_PATH: &str = "docs/moved/guide.md";

/// A repository with two committed, indexed tickets, `TICKET_A` open and
/// `TICKET_CLOSED` closed, and one committed, indexed document,
/// `DOCUMENT`, none of them with an editing context.
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
        items::write(
            &root,
            DOCUMENT_PATH,
            &items::document_source(DOCUMENT, "Guide", ""),
        );
        items::commit(
            &fixture,
            &[&open_ticket, &closed, DOCUMENT_PATH],
            items::COMMITTED_AT,
        );
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

    /// A short code assignment for `id`, with no initials of its own.
    pub fn slug_assign(&self, id: &str, observation: &str) -> Mutation {
        Mutation::TicketSlugAssign(self.slug_assign_input(id, observation))
    }

    pub fn slug_assign_input(&self, id: &str, observation: &str) -> TicketSlugAssignInput {
        TicketSlugAssignInput {
            root: self.root.clone(),
            id: items::item_id(id),
            observation: observation.to_owned(),
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

    /// Checks an envelope that carries no data against its golden fixture
    /// `tests/fixtures/mutation_v1/<name>.json`, as `assert_contract` does:
    /// a refusal, mostly. The root, the commit and the operation ID are
    /// replaced.
    pub fn assert_golden(&self, name: &str, envelope: &impl Serialize) {
        assert_contract_at(name, self.root.to_str().unwrap(), None, &[], envelope);
    }

    /// The same, for an envelope whose data `data_schema` describes.
    pub fn assert_golden_with_data(
        &self,
        name: &str,
        data_schema: &str,
        envelope: &impl Serialize,
    ) {
        let root = self.root.to_str().unwrap();
        assert_contract_at(name, root, Some(data_schema), &[], envelope);
    }

    pub fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    pub fn branch(id: &str) -> String {
        format!("manyhands/ticket/{id}")
    }

    /// The tip of the item's editing branch, a ticket's or a document's,
    /// if it has one.
    pub fn branch_tip(&self, id: &str) -> Option<git2::Oid> {
        let repository = git2::Repository::open(&self.root).unwrap();
        [Self::branch(id), Self::document_branch(id)]
            .iter()
            .find_map(|name| repository.find_branch(name, git2::BranchType::Local).ok())
            .and_then(|branch| branch.get().target())
    }

    pub fn document_branch(id: &str) -> String {
        format!("manyhands/document/{id}")
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

/// A provider with no passphrase to give: nobody is there to ask.
pub struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

/// A sink that keeps what it is sent.
#[derive(Default)]
pub struct Events(pub Vec<ProgressEvent>);

impl ProgressSink for Events {
    fn event(&mut self, event: &ProgressEvent) {
        self.0.push(event.clone());
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
    execute_call(service, request_id, None, mutation, cancel)
}

/// Runs one request that presents the confirmation `confirmation`.
pub fn execute_confirmed(
    service: &RepositoryService,
    request_id: &str,
    confirmation: &str,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    execute_call(
        service,
        request_id,
        Some(ConfirmationId::parse(confirmation).unwrap()),
        mutation,
        &CancellationToken::new(),
    )
}

fn execute_call(
    service: &RepositoryService,
    request_id: &str,
    confirmation: Option<ConfirmationId>,
    mutation: Mutation,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: RequestId::parse(request_id).unwrap(),
            confirmation,
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
        MutationDataDto::Document(_) => panic!("a document's data"),
        MutationDataDto::Remote(_) => panic!("a remote's data"),
        MutationDataDto::Key(_) => panic!("a key's data"),
        MutationDataDto::Operation(_) => panic!("an operation's data"),
        MutationDataDto::Synchronization(_) => panic!("a synchronization's data"),
        MutationDataDto::Identity(_) => panic!("an identity's data"),
        MutationDataDto::Host(_) => panic!("a host's data"),
        MutationDataDto::Repair(_) => panic!("a repair's data"),
        MutationDataDto::Context(_) => panic!("a context's data"),
    }
}

/// What a document request reported about its document.
pub fn document_data(envelope: &Envelope<MutationDataDto>) -> &DocumentMutationDto {
    match envelope.data.as_ref().expect("document data") {
        MutationDataDto::Document(data) => data,
        MutationDataDto::Ticket(_) => panic!("a ticket's data"),
        MutationDataDto::Remote(_) => panic!("a remote's data"),
        MutationDataDto::Key(_) => panic!("a key's data"),
        MutationDataDto::Operation(_) => panic!("an operation's data"),
        MutationDataDto::Synchronization(_) => panic!("a synchronization's data"),
        MutationDataDto::Identity(_) => panic!("an identity's data"),
        MutationDataDto::Host(_) => panic!("a host's data"),
        MutationDataDto::Repair(_) => panic!("a repair's data"),
        MutationDataDto::Context(_) => panic!("a context's data"),
    }
}

impl World {
    pub fn document_draft() -> DocumentDraft {
        DocumentDraft {
            title: TITLE.to_owned(),
            body: BODY.to_owned(),
        }
    }

    /// A create of the document `id` at `path` with the sentinel title
    /// and body.
    pub fn document_create(&self, id: &str, path: &str) -> Mutation {
        Mutation::DocumentCreate(DocumentCreateInput {
            root: self.root.clone(),
            id: items::item_id(id),
            path: PathBuf::from(path),
            draft: Self::document_draft(),
        })
    }

    pub fn document_save_input(&self, id: &str, observation: &str) -> DocumentSaveInput {
        DocumentSaveInput {
            root: self.root.clone(),
            id: items::item_id(id),
            draft: Self::document_draft(),
            observation: Some(observation.to_owned()),
        }
    }

    /// A save of the document `id` with the sentinel title and body.
    pub fn document_save(&self, id: &str, observation: &str) -> Mutation {
        Mutation::DocumentSave(self.document_save_input(id, observation))
    }

    pub fn document_move_input(
        &self,
        id: &str,
        observation: &str,
        destination: &str,
        destination_observation: &str,
    ) -> DocumentMoveInput {
        DocumentMoveInput {
            root: self.root.clone(),
            id: items::item_id(id),
            observation: Some(observation.to_owned()),
            destination_path: PathBuf::from(destination),
            destination_observation: destination_observation.to_owned(),
        }
    }

    pub fn document_move(
        &self,
        id: &str,
        observation: &str,
        destination: &str,
        destination_observation: &str,
    ) -> Mutation {
        Mutation::DocumentMove(self.document_move_input(
            id,
            observation,
            destination,
            destination_observation,
        ))
    }

    /// The token `observe_path` gives for `path` in the context the item
    /// `id` is read from now.
    pub fn observe(&self, id: &str, path: &str) -> String {
        let repo = self.resolved();
        let item = self.service.show_item(&repo, &items::item_id(id)).unwrap();
        let worktree = PathBuf::from(&item.context.worktree);
        let context = (worktree != self.root).then_some(worktree.as_path());
        self.service
            .observe_path(&repo, context, Path::new(path))
            .unwrap()
            .observation
    }

    /// The file at `path` in the primary worktree.
    pub fn primary_source_at(&self, path: &str) -> String {
        fs::read_to_string(self.root.join(path)).unwrap()
    }

    /// The file at `path` in the item's editing context, if it is there.
    pub fn context_source(&self, id: &str, path: &str) -> Option<String> {
        fs::read_to_string(self.worktree(id).join(path)).ok()
    }

    /// The paths under `docs/` that the commit `commit` holds a file at.
    pub fn documents_at(&self, commit: git2::Oid) -> Vec<String> {
        let repository = git2::Repository::open(&self.root).unwrap();
        let tree = repository.find_commit(commit).unwrap().tree().unwrap();
        let mut paths = Vec::new();
        tree.walk(git2::TreeWalkMode::PreOrder, |directory, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) && directory.starts_with("docs/") {
                paths.push(format!("{directory}{}", entry.name().unwrap()));
            }
            git2::TreeWalkResult::Ok
        })
        .unwrap();
        paths.sort();
        paths
    }
}

/// What a remote request reported about its remote.
pub fn remote_data(envelope: &Envelope<MutationDataDto>) -> &RemoteMutationDto {
    match envelope.data.as_ref().expect("remote data") {
        MutationDataDto::Remote(data) => data,
        MutationDataDto::Document(_) => panic!("a document's data"),
        MutationDataDto::Ticket(_) => panic!("a ticket's data"),
        MutationDataDto::Key(_) => panic!("a key's data"),
        MutationDataDto::Operation(_) => panic!("an operation's data"),
        MutationDataDto::Synchronization(_) => panic!("a synchronization's data"),
        MutationDataDto::Identity(_) => panic!("an identity's data"),
        MutationDataDto::Host(_) => panic!("a host's data"),
        MutationDataDto::Repair(_) => panic!("a repair's data"),
        MutationDataDto::Context(_) => panic!("a context's data"),
    }
}

/// What an abandon reported about the operation it closed.
pub fn operation_data(envelope: &Envelope<MutationDataDto>) -> &OperationMutationDto {
    match envelope.data.as_ref().expect("operation data") {
        MutationDataDto::Operation(data) => data,
        MutationDataDto::Document(_) => panic!("a document's data"),
        MutationDataDto::Ticket(_) => panic!("a ticket's data"),
        MutationDataDto::Remote(_) => panic!("a remote's data"),
        MutationDataDto::Key(_) => panic!("a key's data"),
        MutationDataDto::Synchronization(_) => panic!("a synchronization's data"),
        MutationDataDto::Identity(_) => panic!("an identity's data"),
        MutationDataDto::Host(_) => panic!("a host's data"),
        MutationDataDto::Repair(_) => panic!("a repair's data"),
        MutationDataDto::Context(_) => panic!("a context's data"),
    }
}

/// What a key request reported about its key.
pub fn key_data(envelope: &Envelope<MutationDataDto>) -> &KeyMutationDto {
    match envelope.data.as_ref().expect("key data") {
        MutationDataDto::Key(data) => data,
        MutationDataDto::Document(_) => panic!("a document's data"),
        MutationDataDto::Ticket(_) => panic!("a ticket's data"),
        MutationDataDto::Remote(_) => panic!("a remote's data"),
        MutationDataDto::Operation(_) => panic!("an operation's data"),
        MutationDataDto::Synchronization(_) => panic!("a synchronization's data"),
        MutationDataDto::Identity(_) => panic!("an identity's data"),
        MutationDataDto::Host(_) => panic!("a host's data"),
        MutationDataDto::Repair(_) => panic!("a repair's data"),
        MutationDataDto::Context(_) => panic!("a context's data"),
    }
}

/// What a `repo identity-set` reported about the identity.
pub fn identity_data(envelope: &Envelope<MutationDataDto>) -> &IdentityMutationDto {
    match envelope.data.as_ref().expect("identity data") {
        MutationDataDto::Identity(data) => data,
        MutationDataDto::Document(_) => panic!("a document's data"),
        MutationDataDto::Ticket(_) => panic!("a ticket's data"),
        MutationDataDto::Remote(_) => panic!("a remote's data"),
        MutationDataDto::Key(_) => panic!("a key's data"),
        MutationDataDto::Operation(_) => panic!("an operation's data"),
        MutationDataDto::Synchronization(_) => panic!("a synchronization's data"),
        MutationDataDto::Host(_) => panic!("a host's data"),
        MutationDataDto::Repair(_) => panic!("a repair's data"),
        MutationDataDto::Context(_) => panic!("a context's data"),
    }
}

/// The fingerprint of the public key fixture.
pub const PUBLIC_KEY_FINGERPRINT: &str = "SHA256:kmYcvdi2GkPeWxB6XLjrZB8JHsy2Hm8luHMFp9GMvqk";

/// The passphrase the key scenarios protect a generated key with. It
/// holds a sentinel that no request store and no envelope may keep.
pub const PASSPHRASE: &str = "correct horse SENTINEL-PASSPHRASE-c07a";
pub const PASSPHRASE_SENTINEL: &str = "SENTINEL-PASSPHRASE-c07a";

/// A generation of a key labelled `label` into the key store of `home`,
/// protected by `passphrase` or not protected.
pub fn generate_in(home: &Path, label: &str, passphrase: Option<&str>) -> Mutation {
    Mutation::KeyGenerate(KeyGenerateInput {
        store: KeyStore::for_home(home).unwrap(),
        label: label.to_owned(),
        protection: match passphrase {
            Some(passphrase) => {
                KeyProtection::Passphrase(SecretPassphrase::new(passphrase.to_owned()).unwrap())
            }
            None => KeyProtection::Unencrypted,
        },
    })
}

/// A service with no repository, and a home for key files: the key
/// commands belong to the application.
///
/// As for `World`, every envelope `execute` returns is scanned for the
/// sentinels, and the request stores are scanned when the world is
/// dropped.
pub struct KeyWorld {
    pub service: RepositoryService,
    pub data: tempfile::TempDir,
    pub home: tempfile::TempDir,
}

impl KeyWorld {
    pub fn new() -> Self {
        Self::with_service(|data| RepositoryService::open_at(data).unwrap())
    }

    /// The same world, with the service `open` gives for its data
    /// directory: one with a failure point, say.
    pub fn with_service(open: impl FnOnce(&Path) -> RepositoryService) -> Self {
        let data = tempfile::tempdir().unwrap();
        let service = open(data.path());
        Self {
            service,
            data,
            home: tempfile::tempdir().unwrap(),
        }
    }

    /// Another service over the same data, as a new process would open.
    pub fn reopen(&mut self) {
        self.service = RepositoryService::open_at(self.data.path()).unwrap();
    }

    /// The same data under a service that fails once at `point`.
    pub fn fail_once_at(&mut self, point: manyhands::repository::FailurePoint) {
        self.service = super::FailOnce::at(point).open_service(self.data.path());
    }

    /// Loses the cache: the database, with the request records, the key
    /// journal and the key registry, is deleted, and a service is opened
    /// over what is left. The key store is no part of it.
    pub fn lose_cache(&mut self) {
        for entry in fs::read_dir(self.data.path()).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
                fs::remove_file(&path).unwrap();
            }
        }
        self.reopen();
    }

    /// Runs one request and scans its envelope for the sentinels and for
    /// the passphrase.
    pub fn execute(&self, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        let envelope = execute_with(&self.service, request_id, mutation);
        let value = serde_json::to_value(&envelope).unwrap();
        if let Some((sentinel, location)) = golden::find_sentinel(&value, &[PASSPHRASE_SENTINEL]) {
            panic!("the envelope contains {sentinel:?} at {location}");
        }
        envelope
    }

    pub fn record(&self, request_id: &str) -> Option<RequestRecord> {
        self.service
            .request_record(RequestId::parse(request_id).unwrap())
            .unwrap()
    }

    pub fn request_rows(&self) -> i64 {
        items::index(self.data.path())
            .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
            .unwrap()
    }

    /// The key store of the world's home.
    pub fn store(&self) -> KeyStore {
        KeyStore::for_home(self.home.path()).unwrap()
    }

    /// A generation of a key labelled `label`, protected by `passphrase`
    /// or not protected.
    pub fn generate(&self, label: &str, passphrase: Option<&str>) -> Mutation {
        generate_in(self.home.path(), label, passphrase)
    }

    /// Where the key store keeps the files of generated keys.
    pub fn store_directory(&self) -> PathBuf {
        self.home
            .path()
            .canonicalize()
            .unwrap()
            .join(".ssh")
            .join("manyhands")
    }

    /// The private key files in the key store, by the ID of their key, in
    /// ID order: each generated key has one, named by its ID.
    pub fn private_key_files(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.store_directory()) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| SharedKeyId::parse(name).is_ok())
            .collect();
        names.sort();
        names
    }

    /// How many rows the key journal holds.
    pub fn key_operations(&self) -> i64 {
        items::index(self.data.path())
            .query_row("SELECT COUNT(*) FROM key_material_operations", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    /// Writes a file named `name` in the home and registers it as an
    /// imported key labelled `label`, as the domain registers one.
    pub fn register(&self, name: &str, label: &str) -> SharedKeyId {
        let path = self.home.path().canonicalize().unwrap().join(name);
        fs::write(&path, "not a key\n").unwrap();
        let registered = self
            .service
            .register_shared_key(RegisterSharedKeyRequest {
                label: label.to_owned(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: path,
                public_key_path: None,
            })
            .unwrap();
        match registered {
            RegisterSharedKeyOutcome::Registered(registration) => registration.id,
            RegisterSharedKeyOutcome::SourceAlreadyRegistered { existing } => existing,
        }
    }

    /// Copies the public key fixture into the home under `name`, with a
    /// private key file beside it, and returns the two paths: a source
    /// for `key import` whose fingerprint is `PUBLIC_KEY_FINGERPRINT`.
    pub fn key_files(&self, name: &str) -> (PathBuf, PathBuf) {
        let home = self.home.path().canonicalize().unwrap();
        let private = home.join(name);
        let public = home.join(format!("{name}.pub"));
        fs::write(&private, "not a key\n").unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/shared_key_registry_public_fixture.pub"),
            &public,
        )
        .unwrap();
        (private, public)
    }

    /// The registered keys, each with whether it is selected.
    pub fn keys(&self) -> Vec<(SharedKeyId, bool)> {
        self.service
            .list_shared_keys()
            .unwrap()
            .into_iter()
            .map(|registration| (registration.id, registration.selected))
            .collect()
    }

    /// The selected key, if one is.
    pub fn selected(&self) -> Option<SharedKeyId> {
        self.keys()
            .into_iter()
            .find_map(|(id, selected)| selected.then_some(id))
    }
}

impl Drop for KeyWorld {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert_request_stores_exclude(self.data.path(), &SENTINELS);
            assert_request_stores_exclude(self.data.path(), &[PASSPHRASE_SENTINEL]);
            super::assert_operation_records_hold_no_content(self.data.path());
        }
    }
}

/// The envelope of a `prepare` as one of a preview. The data `prepare`
/// gives that is no preview, of a host approval refused for a host that
/// has a pin, is dropped: a test of it reads the envelope as it is.
pub trait Previewed {
    fn previewed(self) -> Envelope<PreviewDto>;
}

impl Previewed for Envelope<PrepareDataDto> {
    fn previewed(self) -> Envelope<PreviewDto> {
        let Envelope {
            schema_version,
            command,
            request_id,
            operation_id,
            outcome,
            code,
            message,
            scope,
            effects,
            data,
            recovery,
        } = self;
        Envelope {
            schema_version,
            command,
            request_id,
            operation_id,
            outcome,
            code,
            message,
            scope,
            effects,
            data: data.and_then(PrepareDataDto::into_preview),
            recovery,
        }
    }
}

/// The names of the recovery actions an envelope suggests.
pub fn recovery_actions<T>(envelope: &Envelope<T>) -> Vec<&'static str> {
    envelope
        .recovery
        .iter()
        .map(|action| action.action.as_str())
        .collect()
}

/// The names of the recovery actions an envelope suggests before the
/// two every result ends with that left its request's record accepted
/// with the request's own local operation pending: `repo.inspect`, and
/// `operation.abandon` of that operation, the one the envelope names.
/// The same request may still finish the work, which is what the actions
/// before these say; where it cannot, this is the way out.
pub fn recovery_before_own_abandon(envelope: &Envelope<MutationDataDto>) -> Vec<&'static str> {
    let actions = recovery_actions(envelope);
    let (own, tail) = actions.split_at(actions.len().saturating_sub(2));
    assert_eq!(
        tail,
        ["repo.inspect", "operation.abandon"],
        "the request's own pending operation is offered to be abandoned: {actions:?}"
    );
    let abandon = envelope.recovery.last().unwrap();
    assert!(envelope.operation_id.is_some());
    assert_eq!(abandon.operation_id, envelope.operation_id);
    assert_eq!(
        abandon.arguments.get("root").and_then(|root| root.as_str()),
        envelope.scope.repository.as_deref()
    );
    own.to_vec()
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

impl World {
    /// Commits `content` at `path` on the branch checked out in the
    /// worktree `directory`, and writes it there, as someone working in
    /// that worktree with Git would.
    pub fn commit_in(directory: &Path, path: &str, content: &str) -> git2::Oid {
        let file = directory.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, content).unwrap();
        let commit = Self::commit_behind(directory, path, content);
        // The worktree's own index follows, so the file is not shown as
        // modified.
        let repository = git2::Repository::open(directory).unwrap();
        let tree = repository.find_commit(commit).unwrap().tree().unwrap();
        let mut index = repository.index().unwrap();
        index.read_tree(&tree).unwrap();
        index.write().unwrap();
        commit
    }

    /// Commits `content` at `path` on the branch checked out in the
    /// worktree `directory` and leaves the worktree as it is, as a push
    /// into the branch or a commit made with plumbing would. The commit
    /// is built from the head's tree and this one file.
    pub fn commit_behind(directory: &Path, path: &str, content: &str) -> git2::Oid {
        let repository = git2::Repository::open(directory).unwrap();
        let head = repository.head().unwrap().peel_to_commit().unwrap();
        let mut index = git2::Index::new().unwrap();
        index.read_tree(&head.tree().unwrap()).unwrap();
        index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: content.len() as u32,
                id: repository.blob(content.as_bytes()).unwrap(),
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            })
            .unwrap();
        let tree = repository
            .find_tree(index.write_tree_to(&repository).unwrap())
            .unwrap();
        let signature = git2::Signature::new(
            "Someone Else",
            "someone-else@example.invalid",
            &git2::Time::new(items::COMMITTED_AT + 60, 0),
        )
        .unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "by hand",
                &tree,
                &[&head],
            )
            .unwrap()
    }

    /// A commit by hand in the item's editing context.
    pub fn commit_in_context(&self, id: &str, path: &str, content: &str) -> git2::Oid {
        Self::commit_in(&self.worktree(id), path, content)
    }

    /// A commit by hand on primary.
    pub fn commit_on_primary(&self, path: &str, content: &str) -> git2::Oid {
        Self::commit_in(&self.root, path, content)
    }
}

impl World {
    /// Loses the cache: the database, with the request records, the
    /// journals and the registration, is deleted, and a service is opened
    /// over what is left, as the next process would.
    pub fn lose_cache(&mut self) {
        for entry in fs::read_dir(self.data.path()).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
                fs::remove_file(&path).unwrap();
            }
        }
        self.reopen();
    }

    /// Rebuilds the index with the root, which registers the repository
    /// again.
    pub fn rebuild(&self) {
        items::rebuild(&self.service, &self.root);
    }
}

impl World {
    /// Amends the tip of the item's editing branch, as `git commit
    /// --amend` with another message would: the same tree and parent, a
    /// new commit. The worktree is as it was.
    pub fn amend_branch_tip(&self, id: &str) -> git2::Oid {
        let repository = git2::Repository::open(&self.root).unwrap();
        let tip = repository
            .find_commit(self.branch_tip(id).unwrap())
            .unwrap();
        let signature = git2::Signature::new(
            "Someone Else",
            "someone-else@example.invalid",
            &git2::Time::new(items::COMMITTED_AT + 120, 0),
        )
        .unwrap();
        let parents: Vec<git2::Commit<'_>> = tip.parents().collect();
        let parents: Vec<&git2::Commit<'_>> = parents.iter().collect();
        let amended = repository
            .commit(
                None,
                &signature,
                &signature,
                "amended",
                &tip.tree().unwrap(),
                &parents,
            )
            .unwrap();
        repository
            .reference(
                &format!("refs/heads/{}", Self::branch(id)),
                amended,
                true,
                "amend",
            )
            .unwrap();
        amended
    }

    /// Resets the item's editing branch and its worktree to `commit`, as
    /// `git reset --hard` in the worktree would.
    pub fn reset_context(&self, id: &str, commit: git2::Oid) {
        let repository = git2::Repository::open(self.worktree(id)).unwrap();
        let target = repository.find_object(commit, None).unwrap();
        repository
            .reset(&target, git2::ResetType::Hard, None)
            .unwrap();
    }
}

// What the conformance table needs of a fixture of these targets; see
// `support::conformance`.

use super::conformance::{Ask, NO_SUCH_CONFIRMATION, Recorded, Subject};

fn rendered(envelope: &Envelope<MutationDataDto>) -> Vec<u8> {
    serde_json::to_vec(envelope).unwrap()
}

/// Gives `rule` the fixture whose service is `service`, over the data
/// directory `data`, and whose requests `call` makes.
fn drive(
    service: &RepositoryService,
    data: &Path,
    call: &dyn Fn(&str, Ask) -> Envelope<MutationDataDto>,
    outdate: Option<&dyn Fn()>,
    rule: &mut dyn FnMut(&Subject<'_>),
) {
    rule(&Subject {
        outdate,
        call: &|request_id, ask| rendered(&call(request_id, ask)),
        record: &|request_id| {
            service
                .request_record(RequestId::parse(request_id).unwrap())
                .unwrap()
                .map(|record| Recorded {
                    finished: record.state == manyhands::repository::RequestState::Finished,
                    image: format!("{record:?}"),
                })
        },
        request_rows: &|| {
            items::index(data)
                .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
                .unwrap()
        },
    });
}

/// Drives a rule of the conformance table through a command that needs
/// no confirmation. `request` gives the request for `Ask::Same`,
/// `Ask::Changed` and `Ask::Refused`; the request with the wrong consent
/// is `Ask::Same` presenting a confirmation no preview gave.
pub fn drive_plain(
    service: &RepositoryService,
    data: &Path,
    request: &dyn Fn(Ask) -> Mutation,
    rule: &mut dyn FnMut(&Subject<'_>),
) {
    drive(
        service,
        data,
        &|request_id, ask| match ask {
            Ask::Same | Ask::Changed | Ask::Refused => {
                execute_with(service, request_id, request(ask))
            }
            Ask::Unconsented => execute_confirmed(
                service,
                request_id,
                NO_SUCH_CONFIRMATION,
                request(Ask::Same),
            ),
        },
        None,
        rule,
    );
}

/// Drives a rule of the conformance table through a command that needs
/// confirmation. `request` gives the request for `Ask::Same` and
/// `Ask::Changed`. Both present the confirmation of one preview of
/// `Ask::Same`, made when the first of them is asked for. The request
/// that is refused is `Ask::Same` presenting a confirmation no preview
/// gave, and the one with the wrong consent presents none.
///
/// `change` changes what a preview of `Ask::Same` shows, as another
/// actor would: the stale-preview rule takes the preview, calls it, and
/// then presents the preview's confirmation.
pub fn drive_confirmed(
    service: &RepositoryService,
    data: &Path,
    request: &dyn Fn(Ask) -> Mutation,
    change: &dyn Fn(),
    rule: &mut dyn FnMut(&Subject<'_>),
) {
    let confirmation = std::cell::OnceCell::new();
    let previewed = || -> &String {
        confirmation.get_or_init(|| {
            let preview = service.prepare(&request(Ask::Same)).previewed();
            preview
                .data
                .unwrap_or_else(|| panic!("no preview: {}", preview.message))
                .confirmation_id
        })
    };
    drive(
        service,
        data,
        &|request_id, ask| match ask {
            Ask::Same | Ask::Changed => {
                execute_confirmed(service, request_id, previewed(), request(ask))
            }
            Ask::Refused => execute_confirmed(
                service,
                request_id,
                NO_SUCH_CONFIRMATION,
                request(Ask::Same),
            ),
            Ask::Unconsented => execute_with(service, request_id, request(Ask::Same)),
        },
        Some(&|| {
            previewed();
            change();
        }),
        rule,
    );
}

const UNIDENTIFIED_HOME: &str = "MANYHANDS_UNIDENTIFIED_HOME";

/// Runs the calling test, whose path in its test target is `name`, in a
/// process of its own in which no identity is found outside the
/// repository: where libgit2 looks for configuration is process-wide. `false` in the parent, once the child
/// has passed; `true` in the child, which goes on with the test.
pub fn in_a_process_without_an_identity(name: &str) -> bool {
    let Some(home) = std::env::var_os(UNIDENTIFIED_HOME) else {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env(UNIDENTIFIED_HOME, home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "the child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "the child ran no test"
        );
        return false;
    };
    for level in [
        git2::ConfigLevel::System,
        git2::ConfigLevel::Global,
        git2::ConfigLevel::XDG,
        git2::ConfigLevel::ProgramData,
    ] {
        // This exact-test child runs nothing else, so no other use of
        // libgit2 overlaps the change.
        unsafe { git2::opts::set_search_path(level, &home).unwrap() };
    }
    true
}
