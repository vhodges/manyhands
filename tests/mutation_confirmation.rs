//! Two-phase confirmation: `prepare` gives a preview and a confirmation,
//! and `execute` runs a command that needs one only for a request that
//! presents it. Driven through `repo create` and `repo enable`, and
//! then through the three confirmed commands of administration:
//! `remote remove`, `repo remove` and `key delete`.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use manyhands::{
    canonical,
    repository::{
        CONFIRMATION_LIFETIME_SECONDS, CommitIdentity, FailurePoint, IndexRefreshInput,
        KeyDeleteInput, KeyDeletionPreviewDto, KeyDeletionState, LeaseKind, Mutation,
        MutationDataDto, PreviewDetailsDto, PreviewDto, PreviewEffect, PreviewIdentitySource,
        RemoteRemovalPreviewDto, RemoteRemoveInput, RemoteSelectInput, RemoveRegistrationRequest,
        RepoCreateInput, RepoEnableInput, RepoRemoveInput, RepositoryPreviewDto,
        RepositoryRemovalPreviewDto, RepositoryService, RequestId, RequestState, SharedKeyId,
        TicketCreateInput, request_store::RequestRecord,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, Outcome, ResultCode, WriteEffect,
    },
};

mod support;

use support::{
    TestRepository, born_repository, golden,
    items::{self, TICKET_A},
    mutation::{
        self, KeyWorld, PASSPHRASE, PASSPHRASE_SENTINEL, REQUEST_1, REQUEST_2, REQUEST_3,
        TestClock, World, execute_confirmed, execute_with, key_data, recovery_actions,
    },
    schema,
};

const PREVIEW_SCHEMA: &str = "preview.schema.json";
const REMOTE_REMOVAL_SCHEMA: &str = "remote_removal_preview.schema.json";
const REPOSITORY_REMOVAL_SCHEMA: &str = "repository_removal_preview.schema.json";
const KEY_DELETION_SCHEMA: &str = "key_deletion_preview.schema.json";
const OPERATION_ABANDON_SCHEMA: &str = "operation_abandon_preview.schema.json";

/// The published schema of the preview `command` gives.
fn preview_schema(command: &str) -> &'static str {
    match command {
        "remote remove" => REMOTE_REMOVAL_SCHEMA,
        "repo remove" => REPOSITORY_REMOVAL_SCHEMA,
        "key delete" => KEY_DELETION_SCHEMA,
        "operation abandon" => OPERATION_ABANDON_SCHEMA,
        "repo create" | "repo enable" => PREVIEW_SCHEMA,
        // `prepare` gives no preview of any other command.
        other => panic!("{other}: no preview schema"),
    }
}

/// `prepare`, with its envelope checked against the published contract.
fn prepare_with(service: &RepositoryService, mutation: &Mutation) -> Envelope<PreviewDto> {
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
const START: i64 = 1_700_000_000;

/// What a scenario does to a repository or a path before its request.
type ArrangeRepository = fn(&Plain);
type ArrangePath = fn(&Path);

/// The message of the commit enablement makes.
const INITIALIZATION: &str = "Initialize Manyhands";

/// Every file and directory under `directory`, by its path below it: a
/// file with its bytes, a directory with none.
fn tree(directory: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
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
fn global_configuration() -> Option<Vec<u8>> {
    git2::Config::find_global()
        .ok()
        .and_then(|path| fs::read(path).ok())
}

/// What a preview must leave as it found it: the files under `parent`,
/// the repository list and Git's global configuration.
#[derive(Debug, PartialEq)]
struct Surroundings {
    files: BTreeMap<PathBuf, Option<Vec<u8>>>,
    repositories: String,
    global: Option<Vec<u8>>,
}

/// A service over a data directory of its own, timed by a clock the test
/// sets, with nothing registered.
struct Desk {
    service: RepositoryService,
    data: tempfile::TempDir,
    clock: Arc<TestClock>,
}

impl Desk {
    fn new() -> Self {
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
    fn reopen(&mut self) {
        self.service = mutation::service_with_clock(self.data.path(), &self.clock);
    }

    /// Another service over the same data that fails once at `point`.
    fn fail_once_at(&mut self, point: FailurePoint) {
        self.service =
            RepositoryService::open_at_with_failure_point_for_testing(self.data.path(), point)
                .unwrap()
                .with_clock_for_testing(self.clock.clone());
    }

    fn surroundings(&self, parent: &Path) -> Surroundings {
        Surroundings {
            files: tree(parent),
            repositories: serde_json::to_string(&self.service.list_repositories().unwrap())
                .unwrap(),
            global: global_configuration(),
        }
    }

    /// `prepare`, with its envelope checked against the published
    /// contract.
    fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        prepare_with(&self.service, mutation)
    }

    /// A preview that is given, and the confirmation it gives.
    fn preview(&self, mutation: &Mutation) -> (String, PreviewDto) {
        let envelope = self.prepare(mutation);
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    fn execute(
        &self,
        request_id: &str,
        confirmation: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        execute_confirmed(&self.service, request_id, confirmation, mutation)
    }

    fn record(&self, request_id: &str) -> Option<RequestRecord> {
        self.service
            .request_record(RequestId::parse(request_id).unwrap())
            .unwrap()
    }

    fn count(&self, query: &str) -> i64 {
        items::index(self.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    fn request_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM request_records")
    }

    fn confirmation_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM confirmation_records")
    }

    /// The confirmations a request has accepted.
    fn accepted_confirmations(&self) -> i64 {
        self.count(
            "SELECT COUNT(*) FROM confirmation_records WHERE accepted_by_request IS NOT NULL",
        )
    }

    fn pending_journal_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'")
    }

    /// Nothing is left of a request that was rejected: no record, no
    /// confirmation held, and no operation in flight.
    fn assert_nothing_left(&self, request_id: &str) {
        assert_eq!(self.record(request_id), None, "a request record");
        assert_eq!(self.request_rows(), 0, "request records");
        assert_eq!(self.accepted_confirmations(), 0, "a confirmation held");
        assert_eq!(self.pending_journal_rows(), 0, "an operation in flight");
    }

    fn registered(&self, root: &Path) -> bool {
        self.service
            .list_repositories()
            .unwrap()
            .items
            .iter()
            .any(|repository| Path::new(&repository.root) == root)
    }
}

fn identity() -> CommitIdentity {
    CommitIdentity {
        name: "Ada Example".to_owned(),
        email: "ada@example.invalid".to_owned(),
    }
}

/// A Git repository with one commit on `main`, a local identity, and no
/// Manyhands configuration.
struct Plain {
    fixture: TestRepository,
    root: PathBuf,
}

impl Plain {
    fn new() -> Self {
        let fixture = born_repository();
        let root = fixture.root.canonicalize().unwrap();
        Self { fixture, root }
    }

    fn input(&self) -> RepoEnableInput {
        RepoEnableInput {
            root: self.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: None,
        }
    }

    fn enable(&self) -> Mutation {
        Mutation::RepoEnable(self.input())
    }

    fn head(&self) -> git2::Oid {
        self.fixture
            .repository
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
    }

    /// The commits on the checked-out branch that enablement made.
    fn initializations(&self) -> usize {
        initializations(&self.root)
    }

    fn configuration(&self) -> Option<String> {
        fs::read_to_string(self.root.join(canonical::CONFIG_PATH)).ok()
    }

    /// Checks out a new branch at the head: the worktree stays clean.
    fn switch_to(&self, branch: &str) {
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

    fn local_config(&self, key: &str) -> Option<String> {
        local_config(&self.root, key)
    }
}

/// The value the repository's own configuration file holds for `key`.
fn local_config(root: &Path, key: &str) -> Option<String> {
    git2::Config::open(&root.join(".git/config"))
        .unwrap()
        .get_string(key)
        .ok()
}

fn initializations(root: &Path) -> usize {
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

fn repository_preview(preview: &PreviewDto) -> &RepositoryPreviewDto {
    match &preview.details {
        PreviewDetailsDto::Repository(details) => details,
        other => panic!("not a preview of a repository's creation: {other:?}"),
    }
}

fn effects(preview: &PreviewDto) -> Vec<PreviewEffect> {
    preview.effects.iter().map(|entry| entry.effect).collect()
}

/// A request that enabled or created a repository: a success that wrote
/// and committed the configuration, with the commit it names.
fn assert_enabled(envelope: &Envelope<MutationDataDto>, root: &Path) -> String {
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.effects.write, WriteEffect::Written);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(envelope.data, None);
    assert!(envelope.recovery.is_empty());
    let commit = envelope.effects.commit_oid.clone().expect("a commit");
    let repository = git2::Repository::open(root).unwrap();
    let named = repository
        .find_commit(git2::Oid::from_str(&commit).unwrap())
        .unwrap();
    assert_eq!(named.message(), Some(INITIALIZATION));
    assert!(
        named
            .tree()
            .unwrap()
            .get_path(Path::new(canonical::CONFIG_PATH))
            .is_ok()
    );
    commit
}

/// A request that was refused before it was accepted.
fn assert_refused(envelope: &Envelope<MutationDataDto>, code: ResultCode, recovery: &[&str]) {
    assert_eq!(envelope.code, code, "{}", envelope.message);
    assert!(
        matches!(envelope.outcome, Outcome::Blocked | Outcome::Error),
        "{:?}",
        envelope.outcome
    );
    assert_eq!(envelope.operation_id, None);
    assert!(!envelope.effects.is_durable());
    assert_eq!(envelope.effects.commit_oid, None);
    assert_eq!(recovery_actions(envelope), recovery);
}

/// What a confirmed request is offered when it is refused with its
/// record left accepted: its own operation is pending, and refuses every
/// other operation on the repository, a new preview's among them. It is
/// to look at the repository, and then, if what was previewed is not put
/// back, to abandon that operation.
fn assert_offered_its_own_abandon(envelope: &Envelope<MutationDataDto>, what: &str) {
    assert_eq!(
        recovery_actions(envelope),
        ["repo.inspect", "operation.abandon"],
        "{what}"
    );
    let abandon = &envelope.recovery[1];
    assert!(envelope.operation_id.is_some(), "{what}");
    assert_eq!(abandon.operation_id, envelope.operation_id, "{what}");
    assert_eq!(
        abandon.arguments.get("root").and_then(|root| root.as_str()),
        envelope.scope.repository.as_deref(),
        "{what}"
    );
}

// ---------------------------------------------------------------------
// `prepare`
// ---------------------------------------------------------------------

#[test]
fn a_preview_of_repo_enable_shows_what_will_happen_and_writes_one_confirmation() {
    let desk = Desk::new();
    let plain = Plain::new();
    let parent = plain.root.clone();
    let before = desk.surroundings(&parent);

    let (confirmation, preview) = desk.preview(&plain.enable());

    assert_eq!(desk.surroundings(&parent), before);
    assert_eq!(desk.confirmation_rows(), 1);
    assert_eq!(desk.request_rows(), 0);
    assert_eq!(desk.count("SELECT COUNT(*) FROM operation_records"), 0);
    assert_eq!(preview.command, "repo enable");
    assert_eq!(
        preview.scope.repository.as_deref(),
        Some(plain.root.to_str().unwrap())
    );
    assert_eq!(preview.expires_at, "2023-11-14T22:23:20Z");
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::WorktreeExclusionWritten,
            PreviewEffect::ConfigurationWritten,
            PreviewEffect::InitializationCommitted,
            PreviewEffect::RepositoryRegistered,
        ]
    );
    let details = repository_preview(&preview);
    assert_eq!(details.root, plain.root.to_str().unwrap());
    assert_eq!(details.primary_branch, "main");
    assert_eq!(details.slug_prefix, None);
    // The fixture's own identity, which is the repository's.
    assert_eq!(details.identity.name, "Manyhands Test");
    assert_eq!(details.identity.email, "manyhands-test@example.invalid");
    assert_eq!(details.identity.source, PreviewIdentitySource::Repository);
    assert!(!details.identity.written_locally);

    let record = desk
        .service
        .confirmation_record(manyhands::repository::ConfirmationId::parse(&confirmation).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(record.command, "repo enable");
    assert_eq!(record.scope.as_str(), plain.root.to_str().unwrap());
    assert_eq!(record.created_at, START);
    assert_eq!(record.expires_at, START + CONFIRMATION_LIFETIME_SECONDS);
    assert_eq!(record.accepted_by, None);
    assert!(record.observation_digest.is_some());
}

#[test]
fn prepare_for_a_command_that_needs_no_confirmation_is_not_confirmable() {
    let world = World::new();
    let token = world.token(TICKET_A);
    for mutation in [
        world.save(TICKET_A, &token),
        Mutation::IndexRefresh(IndexRefreshInput {
            root: world.root.clone(),
        }),
    ] {
        let envelope = world.service.prepare(&mutation);
        assert_eq!(envelope.code, ResultCode::NotConfirmable);
        assert_eq!(envelope.outcome, Outcome::Error);
        assert_eq!(envelope.command, mutation.command());
        assert_eq!(envelope.data, None);
        assert_eq!(envelope.request_id, None);
        assert!(envelope.recovery.is_empty());
        schema::check_published(
            golden::ENVELOPE_SCHEMA,
            &serde_json::to_value(&envelope).unwrap(),
        )
        .unwrap();
    }
    let rows: i64 = items::index(world.data.path())
        .query_row("SELECT COUNT(*) FROM confirmation_records", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 0);
}

// ---------------------------------------------------------------------
// What `execute` asks of a confirmation
// ---------------------------------------------------------------------

#[test]
fn execute_without_a_confirmation_is_confirmation_required_and_does_nothing() {
    let desk = Desk::new();
    let plain = Plain::new();
    let before = tree(&plain.root);
    // Also with a preview outstanding: it is the request that presents it.
    desk.preview(&plain.enable());

    let refused = execute_with(&desk.service, REQUEST_1, plain.enable());

    assert_refused(
        &refused,
        ResultCode::ConfirmationRequired,
        &["request.prepare"],
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    desk.assert_nothing_left(REQUEST_1);
    assert_eq!(desk.count("SELECT COUNT(*) FROM operation_records"), 0);
    assert_eq!(tree(&plain.root), before);
}

#[test]
fn a_confirmed_request_enables_the_repository_and_is_replayed_at_any_later_time() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let base = plain.head();

    let enabled = desk.execute(REQUEST_1, &confirmation, plain.enable());

    let commit = assert_enabled(&enabled, &plain.root);
    assert_eq!(enabled.scope.branch.as_deref(), Some("main"));
    assert_eq!(plain.head().to_string(), commit);
    assert_eq!(
        plain
            .fixture
            .repository
            .find_commit(plain.head())
            .unwrap()
            .parent_id(0)
            .unwrap(),
        base
    );
    assert_eq!(
        plain.configuration().as_deref(),
        Some(support::config_source().as_str())
    );
    assert!(desk.registered(&plain.root));
    let record = desk.record(REQUEST_1).unwrap();
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(
        record.confirmation.map(|id| id.to_string()),
        Some(confirmation.clone())
    );
    assert_eq!(desk.pending_journal_rows(), 0);

    // A day later, with the confirmation long expired, and whatever the
    // retry presents: the same result, and nothing runs.
    desk.clock.set(START + 86_400);
    let again = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(again, enabled);
    let without = execute_with(&desk.service, REQUEST_1, plain.enable());
    assert_eq!(without, enabled);
    assert_eq!(plain.initializations(), 1);
}

#[test]
fn a_confirmation_that_no_record_holds_is_confirmation_not_found() {
    let desk = Desk::new();
    let plain = Plain::new();
    let refused = desk.execute(REQUEST_1, "01ARZ3NDEKTSV4RRFFQ69G5FN9", plain.enable());
    assert_refused(
        &refused,
        ResultCode::ConfirmationNotFound,
        &["request.prepare"],
    );
    desk.assert_nothing_left(REQUEST_1);
    assert_eq!(plain.configuration(), None);
}

/// The database goes, and the confirmation with it: the repository is
/// as it was, and a new preview is what is needed.
#[test]
fn a_confirmation_is_not_found_once_the_cache_is_lost() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    for entry in fs::read_dir(desk.data.path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
            fs::remove_file(&path).unwrap();
        }
    }
    desk.reopen();

    let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_refused(
        &refused,
        ResultCode::ConfirmationNotFound,
        &["request.prepare"],
    );
    desk.assert_nothing_left(REQUEST_1);
    assert_eq!(plain.configuration(), None);
    // The new preview and a request with it do the work.
    let (confirmation, _) = desk.preview(&plain.enable());
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
}

#[test]
fn another_input_another_repository_and_another_command_are_confirmation_mismatch() {
    let desk = Desk::new();
    let plain = Plain::new();
    let other = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let before = (tree(&plain.root), tree(&other.root));

    let changed: Vec<(&str, Mutation)> = vec![
        (
            "a prefix",
            Mutation::RepoEnable(RepoEnableInput {
                slug_prefix: Some("mh".to_owned()),
                ..plain.input()
            }),
        ),
        (
            "an identity",
            Mutation::RepoEnable(RepoEnableInput {
                identity: Some(identity()),
                ..plain.input()
            }),
        ),
        (
            "another branch",
            Mutation::RepoEnable(RepoEnableInput {
                primary_branch: "trunk".to_owned(),
                ..plain.input()
            }),
        ),
        ("another repository", other.enable()),
        (
            "another command",
            Mutation::RepoCreate(RepoCreateInput {
                root: plain.root.clone(),
                primary_branch: "main".to_owned(),
                identity: None,
                slug_prefix: None,
            }),
        ),
    ];
    for (what, mutation) in changed {
        let refused = desk.execute(REQUEST_1, &confirmation, mutation);
        assert_refused(&refused, ResultCode::ConfirmationMismatch, &[]);
        assert_eq!(refused.outcome, Outcome::Error, "{what}");
        desk.assert_nothing_left(REQUEST_1);
    }
    assert_eq!((tree(&plain.root), tree(&other.root)), before);
    // The confirmation is still there for the request it is of.
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
}

#[test]
fn a_confirmation_lasts_ten_minutes_and_a_clock_set_back_does_not_extend_it() {
    // At ten minutes less one second it is accepted.
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.clock.set(START + CONFIRMATION_LIFETIME_SECONDS - 1);
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );

    // At ten minutes, and with the clock set before its creation, it has
    // expired.
    for now in [
        START + CONFIRMATION_LIFETIME_SECONDS,
        START + 86_400,
        START - 1,
        0,
    ] {
        let desk = Desk::new();
        let plain = Plain::new();
        let (confirmation, _) = desk.preview(&plain.enable());
        let before = tree(&plain.root);
        desk.clock.set(now);

        let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());

        assert_refused(
            &refused,
            ResultCode::ConfirmationExpired,
            &["request.prepare"],
        );
        assert_eq!(refused.outcome, Outcome::Blocked, "at {now}");
        desk.assert_nothing_left(REQUEST_1);
        assert_eq!(tree(&plain.root), before, "at {now}");
    }
}

#[test]
fn a_second_request_presenting_an_accepted_confirmation_is_confirmation_used() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let enabled = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_enabled(&enabled, &plain.root);
    let after = tree(&plain.root);

    let refused = desk.execute(REQUEST_2, &confirmation, plain.enable());

    assert_refused(&refused, ResultCode::ConfirmationUsed, &[]);
    assert_eq!(desk.record(REQUEST_2), None);
    assert_eq!(desk.request_rows(), 1);
    assert_eq!(tree(&plain.root), after);
    assert_eq!(plain.initializations(), 1);
}

/// The same holds while the first request is still accepted: the
/// confirmation is its own from the moment it is accepted.
#[test]
fn a_confirmation_an_unfinished_request_holds_is_confirmation_used_for_another() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    let refused = desk.execute(REQUEST_2, &confirmation, plain.enable());

    assert_refused(&refused, ResultCode::ConfirmationUsed, &[]);
    assert_eq!(plain.configuration(), None);
}

/// Between the preview and the request, in turn: the checked-out branch
/// changes, the worktree becomes dirty, a commit is added, and a file
/// is modified. Each is refused as a change from elsewhere,
/// whatever check of the command's own it would also fail.
#[test]
fn a_stale_preview_of_repo_enable_is_external_change() {
    let changes: [(&str, ArrangeRepository); 4] = [
        ("the checked-out branch changes", |plain| {
            plain.switch_to("other")
        }),
        ("the worktree becomes dirty", |plain| {
            fs::write(plain.root.join("untracked.txt"), "new\n").unwrap()
        }),
        ("a commit is added", |plain| {
            World::commit_in(&plain.root, "added.txt", "added\n");
        }),
        ("a file is modified", |plain| {
            fs::write(plain.root.join("fixture.txt"), "changed\n").unwrap()
        }),
    ];
    for (what, change) in changes {
        let desk = Desk::new();
        let plain = Plain::new();
        let (confirmation, _) = desk.preview(&plain.enable());
        change(&plain);
        let before = tree(&plain.root);

        let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());

        assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
        assert_eq!(refused.outcome, Outcome::Blocked, "{what}");
        desk.assert_nothing_left(REQUEST_1);
        assert_eq!(desk.count("SELECT COUNT(*) FROM operation_records"), 0);
        assert_eq!(tree(&plain.root), before, "{what}");
        assert_eq!(plain.configuration(), None, "{what}");
        assert!(!desk.registered(&plain.root), "{what}");
    }
}

/// A change that is undone before the request leaves the preview as good
/// as it was: the digest is of the state, not of a moment.
#[test]
fn a_preview_holds_while_the_state_it_showed_is_the_state_there_is() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    fs::write(plain.root.join("untracked.txt"), "new\n").unwrap();
    assert_eq!(
        desk.execute(REQUEST_1, &confirmation, plain.enable()).code,
        ResultCode::ExternalChange
    );
    fs::remove_file(plain.root.join("untracked.txt")).unwrap();

    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
}

// ---------------------------------------------------------------------
// The joint transaction, and the release of a confirmation
// ---------------------------------------------------------------------

#[test]
fn the_confirmation_and_the_request_record_are_written_in_one_transaction() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let before = tree(&plain.root);
    desk.fail_once_at(FailurePoint::AfterConfirmationAcceptance);

    let failed = desk.execute(REQUEST_1, &confirmation, plain.enable());

    // The failure fell between the two writes, and neither stands.
    assert_eq!(failed.code, ResultCode::InternalError);
    assert_eq!(failed.outcome, Outcome::Error);
    desk.assert_nothing_left(REQUEST_1);
    assert_eq!(desk.confirmation_rows(), 1);
    assert_eq!(tree(&plain.root), before);
    // So the same request, and the same confirmation, are as before.
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
    assert_eq!(desk.accepted_confirmations(), 1);
}

/// A request that is rejected after it accepted its confirmation leaves
/// no record and holds no confirmation: the same request, and a new one,
/// can use it until it expires.
#[test]
fn a_request_rejected_under_the_lease_releases_its_confirmation() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let before = tree(&plain.root);
    // After the request is accepted and before its domain call, another
    // branch is checked out: the domain finds it under its lease.
    let switch = |desk: &Desk| {
        let root = plain.root.clone();
        desk.service.set_request_hook_for_testing(move || {
            let repository = git2::Repository::open(&root).unwrap();
            let head = repository.head().unwrap().peel_to_commit().unwrap();
            if repository
                .find_branch("other", git2::BranchType::Local)
                .is_err()
            {
                repository.branch("other", &head, false).unwrap();
            }
            repository.set_head("refs/heads/other").unwrap();
        });
    };
    let back = || {
        plain
            .fixture
            .repository
            .set_head("refs/heads/main")
            .unwrap()
    };

    for request in [REQUEST_1, REQUEST_1, REQUEST_2] {
        switch(&desk);
        let rejected = desk.execute(request, &confirmation, plain.enable());
        assert_eq!(rejected.code, ResultCode::WrongBranch, "{request}");
        assert_eq!(rejected.outcome, Outcome::Blocked);
        assert!(rejected.recovery.is_empty());
        desk.assert_nothing_left(request);
        // On the other branch what was previewed is not what is there.
        assert_eq!(
            desk.execute(REQUEST_3, &confirmation, plain.enable()).code,
            ResultCode::ExternalChange
        );
        back();
        assert_eq!(tree(&plain.root), {
            let mut expected = before.clone();
            expected.extend(
                tree(&plain.root)
                    .into_iter()
                    .filter(|(path, _)| path.starts_with(".git")),
            );
            expected
        });
    }

    // Until it expires: a second before, a new request still uses it.
    desk.clock.set(START + CONFIRMATION_LIFETIME_SECONDS - 1);
    assert_enabled(
        &desk.execute(REQUEST_3, &confirmation, plain.enable()),
        &plain.root,
    );
    assert_eq!(plain.initializations(), 1);
}

/// The release does not extend the confirmation: it still runs from its
/// creation.
#[test]
fn a_released_confirmation_expires_when_it_would_have() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let root = plain.root.clone();
    desk.service.set_request_hook_for_testing(move || {
        fs::write(root.join("untracked.txt"), "new\n").unwrap();
    });
    desk.clock.set(START + CONFIRMATION_LIFETIME_SECONDS - 1);
    let rejected = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(rejected.code, ResultCode::WorktreeNotClean);
    desk.assert_nothing_left(REQUEST_1);
    fs::remove_file(plain.root.join("untracked.txt")).unwrap();

    desk.clock.set(START + CONFIRMATION_LIFETIME_SECONDS);
    let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_refused(
        &refused,
        ResultCode::ConfirmationExpired,
        &["request.prepare"],
    );
    assert_eq!(plain.configuration(), None);
}

// ---------------------------------------------------------------------
// Retries of an accepted request
// ---------------------------------------------------------------------

/// A request is accepted and interrupted after its initialization
/// commit. An hour later, long after its confirmation's ten minutes, the
/// same request completes: no new preview, and no second commit.
#[test]
fn an_accepted_consent_is_honored_after_its_ten_minutes_for_repo_enable() {
    for point in [
        // The hand-off to the index fails: the row is left pending.
        FailurePoint::BeforeIndexTransactionCommit,
        // The call returns and its result is lost: the row has completed.
        FailurePoint::BeforeRequestSettlement,
    ] {
        let mut desk = Desk::new();
        let plain = Plain::new();
        let (confirmation, _) = desk.preview(&plain.enable());
        desk.fail_once_at(point);

        let interrupted = desk.execute(REQUEST_1, &confirmation, plain.enable());

        assert!(interrupted.outcome != Outcome::Success, "{point:?}");
        assert_eq!(plain.initializations(), 1, "{point:?}");
        let commit = plain.head();
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted),
            "{point:?}"
        );

        desk.reopen();
        desk.clock.set(START + 3600);
        // Whatever the retry presents: the record holds its consent.
        let finished = execute_with(&desk.service, REQUEST_1, plain.enable());

        assert_eq!(
            assert_enabled(&finished, &plain.root),
            commit.to_string(),
            "{point:?}"
        );
        assert_eq!(plain.head(), commit, "{point:?}");
        assert_eq!(plain.initializations(), 1, "{point:?}");
        assert_eq!(desk.confirmation_rows(), 1, "{point:?}");
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished)
        );
        assert_eq!(desk.pending_journal_rows(), 0, "{point:?}");
        assert!(desk.registered(&plain.root), "{point:?}");
        // And it is replayed from then on.
        assert_eq!(
            desk.execute(REQUEST_1, &confirmation, plain.enable()),
            finished
        );
    }
}

/// A hand-off that failed is reported as what it is, with the commit.
#[test]
fn a_failed_hand_off_after_the_commit_is_partial_with_the_commit() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeIndexTransactionCommit);

    let pending = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(pending.code, ResultCode::DiscoveryPending);
    assert_eq!(pending.outcome, Outcome::Partial);
    assert_eq!(pending.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(pending.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(pending.effects.commit_oid, Some(plain.head().to_string()));
    assert_eq!(recovery_actions(&pending), ["operation.resume"]);
}

/// A request is accepted and stops before its first effect. The state
/// its preview showed changes. The retry observes again and is refused:
/// the consent was to what was shown.
#[test]
fn a_retry_before_any_effect_observes_again() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    let died = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    fs::write(plain.root.join("untracked.txt"), "new\n").unwrap();
    let before = tree(&plain.root);

    let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert_eq!(tree(&plain.root), before);
    assert_eq!(plain.configuration(), None);
    // Nothing had started, so nothing is left: a new preview and a new
    // request, or the same one, go on from here.
    desk.assert_nothing_left(REQUEST_1);
    fs::remove_file(plain.root.join("untracked.txt")).unwrap();
    let (confirmation, _) = desk.preview(&plain.enable());
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
}

/// The same interruption with nothing changed: the retry is honored,
/// also after the ten minutes.
#[test]
fn a_retry_before_any_effect_runs_when_nothing_has_changed() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    desk.execute(REQUEST_1, &confirmation, plain.enable());
    desk.clock.set(START + 3600);

    let finished = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_enabled(&finished, &plain.root);
    assert_eq!(plain.initializations(), 1);
}

/// An enablement that fails before its commit undoes what it wrote and
/// leaves nothing: its record is deleted and its confirmation released,
/// as for any request rejected with nothing written. The request is then
/// a new one, and its confirmation is judged as a new request's is.
#[test]
fn a_request_that_was_undone_holds_no_consent() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let before = tree(&plain.root);
    desk.fail_once_at(FailurePoint::BeforeInitializationCommit);

    let failed = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(failed.code, ResultCode::InternalError);
    assert_eq!(failed.outcome, Outcome::Error);
    desk.assert_nothing_left(REQUEST_1);
    assert_eq!(plain.configuration(), None);
    assert_eq!(
        tree(&plain.root)
            .into_iter()
            .filter(|(path, _)| !path.starts_with(".git"))
            .collect::<Vec<_>>(),
        before
            .into_iter()
            .filter(|(path, _)| !path.starts_with(".git"))
            .collect::<Vec<_>>()
    );

    desk.clock.set(START + 3600);
    let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_refused(
        &refused,
        ResultCode::ConfirmationExpired,
        &["request.prepare"],
    );
}

#[test]
fn a_failed_registry_write_is_registration_pending_and_its_retry_registers() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRegistryWrite);

    let pending = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(pending.code, ResultCode::RegistrationPending);
    assert_eq!(pending.outcome, Outcome::Partial);
    assert_eq!(pending.effects.write, WriteEffect::Written);
    assert_eq!(pending.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(pending.effects.discovery, DiscoveryEffect::Pending);
    let commit = plain.head();
    assert_eq!(pending.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(recovery_actions(&pending), ["request.retry"]);
    assert!(!desk.registered(&plain.root));
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    desk.reopen();
    desk.clock.set(START + 3600);
    let finished = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(assert_enabled(&finished, &plain.root), commit.to_string());
    assert!(desk.registered(&plain.root));
    assert_eq!(plain.initializations(), 1);
    assert_eq!(desk.pending_journal_rows(), 0);
}

/// A request under an unfinished one's ID with another input is refused,
/// and says what the unfinished one has done.
#[test]
fn a_changed_request_under_an_unfinished_enablement_reports_its_commit() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRegistryWrite);
    desk.execute(REQUEST_1, &confirmation, plain.enable());

    let mismatched = desk.execute(
        REQUEST_1,
        &confirmation,
        Mutation::RepoEnable(RepoEnableInput {
            slug_prefix: Some("mh".to_owned()),
            ..plain.input()
        }),
    );

    assert_eq!(mismatched.code, ResultCode::RequestMismatch);
    assert_eq!(mismatched.outcome, Outcome::Partial);
    assert_eq!(
        mismatched.effects.commit_oid,
        Some(plain.head().to_string())
    );
    assert_eq!(plain.initializations(), 1);
}

// ---------------------------------------------------------------------
// What `prepare` refuses, and the repository that is enabled already
// ---------------------------------------------------------------------

#[test]
fn repo_enable_on_a_wrong_branch_is_wrong_branch() {
    let desk = Desk::new();
    let plain = Plain::new();
    let wrong = Mutation::RepoEnable(RepoEnableInput {
        primary_branch: "trunk".to_owned(),
        ..plain.input()
    });
    let before = desk.surroundings(&plain.root);

    let refused = desk.prepare(&wrong);

    assert_eq!(refused.code, ResultCode::WrongBranch);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.data, None);
    assert!(refused.recovery.is_empty());
    assert_eq!(desk.confirmation_rows(), 0);
    assert_eq!(desk.surroundings(&plain.root), before);
}

#[test]
fn prepare_refuses_what_the_command_would_be_refused_for() {
    let desk = Desk::new();
    let cases: [(&str, ArrangeRepository, ResultCode); 3] = [
        (
            "a dirty worktree",
            |plain| fs::write(plain.root.join("untracked.txt"), "new\n").unwrap(),
            ResultCode::WorktreeNotClean,
        ),
        (
            "a detached head",
            |plain| {
                plain
                    .fixture
                    .repository
                    .set_head_detached(plain.head())
                    .unwrap()
            },
            ResultCode::WrongBranch,
        ),
        (
            "a configuration that cannot be read",
            |plain| {
                fs::create_dir_all(plain.root.join(".manyhands")).unwrap();
                fs::write(plain.root.join(canonical::CONFIG_PATH), "not = [toml").unwrap();
                fs::write(plain.root.join(".git/info/exclude"), ".manyhands/\n").unwrap();
            },
            ResultCode::InvalidConfiguration,
        ),
    ];
    for (what, arrange, code) in cases {
        let plain = Plain::new();
        arrange(&plain);
        let refused = desk.prepare(&plain.enable());
        assert_eq!(refused.code, code, "{what}");
        assert_eq!(refused.data, None, "{what}");
    }
    for (what, input) in [
        (
            "a prefix that is no prefix",
            RepoEnableInput {
                slug_prefix: Some("Not Valid".to_owned()),
                ..Plain::new().input()
            },
        ),
        (
            "an identity with no name",
            RepoEnableInput {
                identity: Some(CommitIdentity {
                    name: String::new(),
                    email: "ada@example.invalid".to_owned(),
                }),
                ..Plain::new().input()
            },
        ),
        (
            "a branch name Git does not take",
            RepoEnableInput {
                primary_branch: "not a branch".to_owned(),
                ..Plain::new().input()
            },
        ),
    ] {
        let refused = desk.prepare(&Mutation::RepoEnable(input));
        assert_eq!(refused.code, ResultCode::InvalidInput, "{what}");
    }
    let nowhere = tempfile::tempdir().unwrap();
    let refused = desk.prepare(&Mutation::RepoEnable(RepoEnableInput {
        root: nowhere.path().to_owned(),
        ..Plain::new().input()
    }));
    assert_eq!(refused.code, ResultCode::NotRepository);
    assert_eq!(desk.confirmation_rows(), 0);
}

#[test]
fn prepare_on_an_enabled_repository_is_a_no_op_with_no_confirmation() {
    let desk = Desk::new();
    let plain = Plain::new();
    let prefixed = |prefix: &str| {
        Mutation::RepoEnable(RepoEnableInput {
            slug_prefix: Some(prefix.to_owned()),
            ..plain.input()
        })
    };
    let (confirmation, _) = desk.preview(&prefixed("mh"));
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, prefixed("mh")),
        &plain.root,
    );
    let before = desk.surroundings(&plain.root);

    for mutation in [plain.enable(), prefixed("mh")] {
        let noop = desk.prepare(&mutation);
        assert_eq!(noop.code, ResultCode::AlreadyApplied);
        assert_eq!(noop.outcome, Outcome::Noop);
        assert_eq!(noop.data, None);
        assert!(noop.recovery.is_empty());
    }
    // Nothing would write another prefix.
    let refused = desk.prepare(&prefixed("other"));
    assert_eq!(refused.code, ResultCode::InvalidInput);
    assert_eq!(refused.outcome, Outcome::Error);
    assert_eq!(refused.data, None);

    assert_eq!(desk.confirmation_rows(), 1, "only the first preview's");
    assert_eq!(desk.surroundings(&plain.root), before);
}

/// A prefix is asked for where the repository holds none: nothing would
/// write it either.
#[test]
fn a_prefix_for_a_repository_enabled_without_one_is_invalid_input() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );

    let refused = desk.prepare(&Mutation::RepoEnable(RepoEnableInput {
        slug_prefix: Some("mh".to_owned()),
        ..plain.input()
    }));

    assert_eq!(refused.code, ResultCode::InvalidInput);
}

// ---------------------------------------------------------------------
// The prefix and the identity
// ---------------------------------------------------------------------

#[test]
fn a_prefix_is_in_the_preview_and_in_the_committed_configuration() {
    let desk = Desk::new();
    let plain = Plain::new();
    let enable = || {
        Mutation::RepoEnable(RepoEnableInput {
            slug_prefix: Some("mh".to_owned()),
            ..plain.input()
        })
    };

    let (confirmation, preview) = desk.preview(&enable());
    assert_eq!(
        repository_preview(&preview).slug_prefix.as_deref(),
        Some("mh")
    );
    let enabled = desk.execute(REQUEST_1, &confirmation, enable());

    assert_enabled(&enabled, &plain.root);
    let expected = "format_version = 1\nprimary_branch = \"main\"\nticket_slug_prefix = \"mh\"\n";
    assert_eq!(plain.configuration().as_deref(), Some(expected));
    let committed = plain
        .fixture
        .repository
        .find_commit(plain.head())
        .unwrap()
        .tree()
        .unwrap()
        .get_path(Path::new(canonical::CONFIG_PATH))
        .unwrap()
        .to_object(&plain.fixture.repository)
        .unwrap();
    assert_eq!(committed.as_blob().unwrap().content(), expected.as_bytes());
}

#[test]
fn a_supplied_identity_is_previewed_and_written_to_the_repository_alone() {
    let desk = Desk::new();
    let plain = Plain::new();
    let enable = || {
        Mutation::RepoEnable(RepoEnableInput {
            identity: Some(identity()),
            ..plain.input()
        })
    };
    let global = global_configuration();

    let (confirmation, preview) = desk.preview(&enable());

    let details = repository_preview(&preview);
    assert_eq!(details.identity.name, "Ada Example");
    assert_eq!(details.identity.email, "ada@example.invalid");
    assert_eq!(details.identity.source, PreviewIdentitySource::Supplied);
    assert!(details.identity.written_locally);
    assert_eq!(effects(&preview)[0], PreviewEffect::IdentityWritten);
    // The preview wrote none of it.
    assert_eq!(
        plain.local_config("user.name").as_deref(),
        Some("Manyhands Test")
    );

    let enabled = desk.execute(REQUEST_1, &confirmation, enable());

    assert_enabled(&enabled, &plain.root);
    assert_eq!(
        plain.local_config("user.name").as_deref(),
        Some("Ada Example")
    );
    assert_eq!(
        plain.local_config("user.email").as_deref(),
        Some("ada@example.invalid")
    );
    let commit = plain.fixture.repository.find_commit(plain.head()).unwrap();
    assert_eq!(commit.author().name(), Some("Ada Example"));
    assert_eq!(global_configuration(), global);
}

/// The variable that tells a test it is the child of itself, and where
/// that child's Git looks for configuration outside a repository.
const GIT_HOME: &str = "MANYHANDS_TEST_GIT_HOME";

/// Runs the calling test, `name`, in a process of its own in which the
/// only configuration outside a repository is `<home>/.gitconfig`, which
/// does not exist until the test writes it: where libgit2 looks is
/// process-wide. `None` in the parent, once the child has passed; the
/// home in the child, which goes on with the test.
fn in_a_process_with_its_own_git_home(name: &str) -> Option<PathBuf> {
    let Some(home) = std::env::var_os(GIT_HOME) else {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env(GIT_HOME, home.path())
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
        return None;
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
    Some(PathBuf::from(home))
}

/// Writes the global configuration of a test's own Git home.
fn write_global_identity(home: &Path) -> Vec<u8> {
    let source = "[user]\n\tname = Global Person\n\temail = global@example.invalid\n";
    fs::write(home.join(".gitconfig"), source).unwrap();
    assert_eq!(global_configuration().as_deref(), Some(source.as_bytes()));
    source.as_bytes().to_vec()
}

#[test]
fn repo_enable_with_a_global_identity_names_its_source_and_writes_none() {
    let Some(home) = in_a_process_with_its_own_git_home(
        "repo_enable_with_a_global_identity_names_its_source_and_writes_none",
    ) else {
        return;
    };
    let global = write_global_identity(&home);
    let desk = Desk::new();
    let fixture = support::repository_without_local_identity();
    let plain = Plain {
        root: fixture.root.canonicalize().unwrap(),
        fixture,
    };

    let (confirmation, preview) = desk.preview(&plain.enable());

    let details = repository_preview(&preview);
    assert_eq!(details.identity.name, "Global Person");
    assert_eq!(details.identity.email, "global@example.invalid");
    assert_eq!(details.identity.source, PreviewIdentitySource::Global);
    assert!(!details.identity.written_locally);
    assert!(!effects(&preview).contains(&PreviewEffect::IdentityWritten));

    let enabled = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_enabled(&enabled, &plain.root);
    let commit = plain.fixture.repository.find_commit(plain.head()).unwrap();
    assert_eq!(commit.author().name(), Some("Global Person"));
    assert_eq!(plain.local_config("user.name"), None);
    assert_eq!(plain.local_config("user.email"), None);
    assert_eq!(global_configuration(), Some(global));
}

#[test]
fn repo_enable_with_no_identity_at_all_is_identity_required() {
    let Some(_home) = in_a_process_with_its_own_git_home(
        "repo_enable_with_no_identity_at_all_is_identity_required",
    ) else {
        return;
    };
    let desk = Desk::new();
    let fixture = support::repository_without_local_identity();
    let plain = Plain {
        root: fixture.root.canonicalize().unwrap(),
        fixture,
    };
    let before = desk.surroundings(&plain.root);

    let refused = desk.prepare(&plain.enable());

    assert_eq!(refused.code, ResultCode::IdentityRequired);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.data, None);
    // The repository is there to set one on.
    let actions: Vec<_> = refused
        .recovery
        .iter()
        .map(|action| action.action.as_str())
        .collect();
    assert_eq!(actions, ["repo.identity_set"]);
    assert_eq!(
        refused.recovery[0].arguments["root"],
        plain.root.to_str().unwrap()
    );
    assert_eq!(desk.confirmation_rows(), 0);
    assert_eq!(desk.surroundings(&plain.root), before);
    assert_eq!(global_configuration(), None);
}

// ---------------------------------------------------------------------
// No lock, and nothing kept that should not be
// ---------------------------------------------------------------------

/// The process that holds a lease for `an_outstanding_preview...`.
#[test]
fn common_git_lease_child() {
    let Ok(root) = std::env::var("MANYHANDS_LEASE_ROOT") else {
        return;
    };
    let variable = |name: &str| PathBuf::from(std::env::var(name).unwrap());
    let kind = LeaseKind::parse(&std::env::var("MANYHANDS_LEASE_KIND").unwrap()).unwrap();
    let _holder = RepositoryService::hold_lease_for_testing(
        Path::new(&root),
        &variable("MANYHANDS_LEASE_DATA_DIRECTORY"),
        kind,
    )
    .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(variable("MANYHANDS_LEASE_READY"))
        .unwrap();
    let release = variable("MANYHANDS_LEASE_RELEASE");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !release.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for lease release"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// `prepare` takes no lease: it gives its preview while another process
/// holds the lease of the very repository it previews, which an
/// `execute` on that repository is refused for.
#[test]
fn prepare_takes_no_lease_of_the_repository_it_previews() {
    let desk = Desk::new();
    let plain = Plain::new();
    let site = Site::new();

    let held = support::hold_lease_in_child(&plain.root, desk.data.path(), LeaseKind::Repository);
    let (confirmation, _) = desk.preview(&plain.enable());
    // The lease is in the way of the command itself, so it would have
    // been in the way of a preview that asked for it.
    let busy = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(busy.code, ResultCode::Busy);
    desk.assert_nothing_left(REQUEST_1);
    assert_eq!(plain.configuration(), None);
    held.release();

    let held = support::hold_lease_in_child(&site.root, desk.data.path(), LeaseKind::Bootstrap);
    let (create_confirmation, _) = desk.preview(&site.create());
    assert_eq!(
        desk.execute(REQUEST_2, &create_confirmation, site.create())
            .code,
        ResultCode::Busy
    );
    assert!(!site.root.exists());
    held.release();

    // Both previews are as good as they were.
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
    assert_enabled(
        &desk.execute(REQUEST_2, &create_confirmation, site.create()),
        &site.root,
    );
}

/// A preview that is outstanding holds nothing on the repository it is
/// of. The repository is enabled by other means while its preview
/// stands, and a save and an `index refresh` on that same repository
/// succeed at once.
#[test]
fn a_save_and_a_refresh_succeed_on_a_repository_whose_preview_is_outstanding() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    assert_eq!(desk.confirmation_rows(), 1);
    assert_eq!(desk.accepted_confirmations(), 0);
    desk.service
        .enable(support::enable_request(&plain.root))
        .unwrap();
    // Enabling commits the configuration without refreshing Git's own
    // index, and a context is not prepared while that path looks changed.
    let mut index = plain.fixture.repository.index().unwrap();
    index
        .read_tree(
            &plain
                .fixture
                .repository
                .find_commit(plain.head())
                .unwrap()
                .tree()
                .unwrap(),
        )
        .unwrap();
    index.write().unwrap();

    let saved = execute_with(
        &desk.service,
        REQUEST_1,
        Mutation::TicketCreate(TicketCreateInput {
            root: plain.root.clone(),
            id: items::item_id(mutation::TICKET_NEW),
            draft: World::draft("A ticket", "Its body.\n"),
            deps: Vec::new(),
            parent: None,
            initials: Some("ab".to_owned()),
        }),
    );
    assert_eq!(saved.outcome, Outcome::Success, "{}", saved.message);
    assert_eq!(saved.effects.checkpoint, CheckpointEffect::Committed);
    let refreshed = execute_with(
        &desk.service,
        REQUEST_2,
        Mutation::IndexRefresh(IndexRefreshInput {
            root: plain.root.clone(),
        }),
    );
    assert_eq!(refreshed.outcome, Outcome::Success, "{}", refreshed.message);

    // The preview is still outstanding, and is of a state that is gone.
    let stale = desk.execute(REQUEST_3, &confirmation, plain.enable());
    assert_refused(&stale, ResultCode::ExternalChange, &["request.prepare"]);
}

const URL_SENTINEL: &str = "SENTINEL-URL-PASSWORD-5b1e";
const BODY_SENTINEL: &str = "SENTINEL-CONFIRMATION-BODY-c44d";

/// A repository whose remote's location holds a password, and that holds
/// a file with a body: neither is in the preview, a confirmation record,
/// a request record or a journal row.
#[test]
fn no_preview_and_no_record_holds_a_remote_s_password_or_a_body() {
    let desk = Desk::new();
    let plain = Plain::new();
    plain
        .fixture
        .repository
        .remote(
            "origin",
            &format!("ssh://git:{URL_SENTINEL}@example.invalid/repository.git"),
        )
        .unwrap();
    World::commit_in(
        &plain.root,
        "docs/guide.md",
        &format!("# Guide\n\n{BODY_SENTINEL}\n"),
    );
    let sentinels = [URL_SENTINEL, BODY_SENTINEL];

    let prepared = desk.prepare(&plain.enable());
    let value = serde_json::to_value(&prepared).unwrap();
    assert_eq!(golden::find_sentinel(&value, &sentinels), None);
    let confirmation = prepared.data.unwrap().confirmation_id;
    mutation::assert_request_stores_exclude(desk.data.path(), &sentinels);

    let enabled = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_enabled(&enabled, &plain.root);
    assert_eq!(
        golden::find_sentinel(&serde_json::to_value(&enabled).unwrap(), &sentinels),
        None
    );
    mutation::assert_request_stores_exclude(desk.data.path(), &sentinels);
    // The scan reads the stores the two left rows in.
    assert_eq!(desk.confirmation_rows(), 1);
    assert_eq!(desk.request_rows(), 1);
    assert!(mutation::find_stored_sentinel(desk.data.path(), &[&confirmation]).is_some());
}

// ---------------------------------------------------------------------
// `repo create`
// ---------------------------------------------------------------------

/// A parent directory and a root below it that does not exist.
struct Site {
    parent: tempfile::TempDir,
    /// The root as the scope key holds it: the canonical parent joined
    /// with the name.
    root: PathBuf,
}

impl Site {
    fn new() -> Self {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().canonicalize().unwrap().join("new");
        Self { parent, root }
    }

    fn input(&self) -> RepoCreateInput {
        RepoCreateInput {
            // As a caller gives it, which need not be canonical.
            root: self.parent.path().join("new"),
            primary_branch: "main".to_owned(),
            identity: Some(identity()),
            slug_prefix: None,
        }
    }

    fn create(&self) -> Mutation {
        Mutation::RepoCreate(self.input())
    }

    fn head(&self) -> git2::Oid {
        git2::Repository::open(&self.root)
            .unwrap()
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
    }

    fn configuration(&self) -> Option<String> {
        fs::read_to_string(self.root.join(canonical::CONFIG_PATH)).ok()
    }
}

#[test]
fn a_preview_of_repo_create_creates_nothing() {
    let desk = Desk::new();
    let site = Site::new();
    let before = desk.surroundings(site.parent.path());

    let (_, preview) = desk.preview(&site.create());

    // No directory, no repository, no registration, no configuration.
    assert_eq!(desk.surroundings(site.parent.path()), before);
    assert!(before.files.is_empty());
    assert!(!site.root.exists());
    assert_eq!(desk.confirmation_rows(), 1);
    assert_eq!(desk.request_rows(), 0);
    assert_eq!(desk.count("SELECT COUNT(*) FROM operation_records"), 0);
    assert_eq!(desk.count("SELECT COUNT(*) FROM repositories"), 0);

    assert_eq!(preview.command, "repo create");
    assert_eq!(
        preview.scope.repository.as_deref(),
        Some(site.root.to_str().unwrap())
    );
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::DirectoryCreated,
            PreviewEffect::RepositoryInitialized,
            PreviewEffect::IdentityWritten,
            PreviewEffect::WorktreeExclusionWritten,
            PreviewEffect::ConfigurationWritten,
            PreviewEffect::InitializationCommitted,
            PreviewEffect::RepositoryRegistered,
        ]
    );
    let details = repository_preview(&preview);
    assert_eq!(details.root, site.root.to_str().unwrap());
    assert_eq!(details.primary_branch, "main");
    assert_eq!(details.identity.source, PreviewIdentitySource::Supplied);
    assert!(details.identity.written_locally);
}

#[test]
fn a_confirmed_request_creates_and_enables_the_repository() {
    let desk = Desk::new();
    let site = Site::new();
    let create = || {
        Mutation::RepoCreate(RepoCreateInput {
            slug_prefix: Some("mh".to_owned()),
            ..site.input()
        })
    };
    let global = global_configuration();
    let (confirmation, preview) = desk.preview(&create());
    assert_eq!(
        repository_preview(&preview).slug_prefix.as_deref(),
        Some("mh")
    );

    let created = desk.execute(REQUEST_1, &confirmation, create());

    let commit = assert_enabled(&created, &site.root);
    assert_eq!(
        created.scope.repository.as_deref(),
        Some(site.root.to_str().unwrap())
    );
    assert_eq!(created.scope.branch.as_deref(), Some("main"));
    assert_eq!(site.head().to_string(), commit);
    assert_eq!(initializations(&site.root), 1);
    assert_eq!(
        site.configuration().as_deref(),
        Some("format_version = 1\nprimary_branch = \"main\"\nticket_slug_prefix = \"mh\"\n")
    );
    assert_eq!(
        local_config(&site.root, "user.name").as_deref(),
        Some("Ada Example")
    );
    assert_eq!(
        local_config(&site.root, "user.email").as_deref(),
        Some("ada@example.invalid")
    );
    assert_eq!(global_configuration(), global);
    assert!(desk.registered(&site.root));
    assert_eq!(desk.pending_journal_rows(), 0);

    // Replayed at any later time, under the root's own name too: the
    // scope key is the same before and after the directory exists.
    desk.clock.set(START + 86_400);
    assert_eq!(desk.execute(REQUEST_1, &confirmation, create()), created);
    let canonical = Mutation::RepoCreate(RepoCreateInput {
        root: site.root.clone(),
        slug_prefix: Some("mh".to_owned()),
        ..site.input()
    });
    assert_eq!(desk.execute(REQUEST_1, &confirmation, canonical), created);
}

/// Without a prefix the committed file is the one enablement has always
/// written.
#[test]
fn a_repository_created_without_a_prefix_has_the_plain_configuration() {
    let desk = Desk::new();
    let site = Site::new();
    let (confirmation, preview) = desk.preview(&site.create());
    assert_eq!(repository_preview(&preview).slug_prefix, None);

    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, site.create()),
        &site.root,
    );

    assert_eq!(
        site.configuration().as_deref(),
        Some(support::config_source().as_str())
    );
}

/// Between the preview and the request: the target appears, a file
/// appears in a target that was an empty directory, and a target that
/// was an empty directory goes.
#[test]
fn a_stale_preview_of_repo_create_is_external_change() {
    let changes: [(&str, bool, ArrangePath); 4] = [
        ("the target appears, empty", false, |root| {
            fs::create_dir(root).unwrap()
        }),
        ("the target appears with a file in it", false, |root| {
            fs::create_dir(root).unwrap();
            fs::write(root.join("notes.txt"), "mine\n").unwrap();
        }),
        ("a file appears in the target directory", true, |root| {
            fs::write(root.join("notes.txt"), "mine\n").unwrap()
        }),
        ("the target directory goes", true, |root| {
            fs::remove_dir(root).unwrap()
        }),
    ];
    for (what, exists, change) in changes {
        let desk = Desk::new();
        let site = Site::new();
        if exists {
            fs::create_dir(&site.root).unwrap();
        }
        let (confirmation, _) = desk.preview(&site.create());
        change(&site.root);
        let before = desk.surroundings(site.parent.path());

        let refused = desk.execute(REQUEST_1, &confirmation, site.create());

        assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
        assert_eq!(refused.outcome, Outcome::Blocked, "{what}");
        desk.assert_nothing_left(REQUEST_1);
        assert_eq!(desk.count("SELECT COUNT(*) FROM operation_records"), 0);
        assert_eq!(desk.surroundings(site.parent.path()), before, "{what}");
    }
}

#[test]
fn repo_create_refuses_a_target_that_is_occupied_or_has_no_parent() {
    let desk = Desk::new();
    let occupied: [(&str, ArrangePath); 2] = [
        ("a file", |root| fs::write(root, "a file\n").unwrap()),
        ("a directory that holds something", |root| {
            fs::create_dir(root).unwrap();
            fs::write(root.join("notes.txt"), "mine\n").unwrap();
        }),
    ];
    for (what, arrange) in occupied {
        let site = Site::new();
        arrange(&site.root);
        let before = desk.surroundings(site.parent.path());
        let refused = desk.prepare(&site.create());
        assert_eq!(refused.code, ResultCode::OccupiedPath, "{what}");
        assert_eq!(refused.outcome, Outcome::Error, "{what}");
        assert_eq!(refused.data, None);
        assert_eq!(desk.surroundings(site.parent.path()), before, "{what}");
    }
    let site = Site::new();
    let refused = desk.prepare(&Mutation::RepoCreate(RepoCreateInput {
        root: site.root.join("missing/new"),
        ..site.input()
    }));
    assert_eq!(refused.code, ResultCode::InvalidPath);
    assert_eq!(desk.confirmation_rows(), 0);
    // And an `execute` of such a request, whatever it presents.
    let refused = desk.execute(
        REQUEST_1,
        "01ARZ3NDEKTSV4RRFFQ69G5FN9",
        Mutation::RepoCreate(RepoCreateInput {
            root: site.root.join("missing/new"),
            ..site.input()
        }),
    );
    assert_eq!(refused.code, ResultCode::InvalidPath);
    desk.assert_nothing_left(REQUEST_1);
}

/// As for `repo enable`: interrupted after the initialization commit,
/// the same request completes an hour later with no new preview and no
/// second commit. Its retry goes through `enable`: the repository is
/// there, and only the first part of a create is its own.
#[test]
fn an_accepted_consent_is_honored_after_its_ten_minutes_for_repo_create() {
    for point in [
        FailurePoint::BeforeIndexTransactionCommit,
        FailurePoint::BeforeRegistryWrite,
        FailurePoint::BeforeRequestSettlement,
    ] {
        let mut desk = Desk::new();
        let site = Site::new();
        let (confirmation, _) = desk.preview(&site.create());
        desk.fail_once_at(point);

        let interrupted = desk.execute(REQUEST_1, &confirmation, site.create());

        assert!(interrupted.outcome != Outcome::Success, "{point:?}");
        assert_eq!(initializations(&site.root), 1, "{point:?}");
        let commit = site.head();
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted),
            "{point:?}"
        );

        desk.reopen();
        desk.clock.set(START + 3600);
        let finished = desk.execute(REQUEST_1, &confirmation, site.create());

        assert_eq!(
            assert_enabled(&finished, &site.root),
            commit.to_string(),
            "{point:?}"
        );
        assert_eq!(site.head(), commit, "{point:?}");
        assert_eq!(initializations(&site.root), 1, "{point:?}");
        assert_eq!(desk.pending_journal_rows(), 0, "{point:?}");
        assert!(desk.registered(&site.root), "{point:?}");
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished)
        );
    }
}

/// A create into a directory that exists and is empty fails once its
/// repository is initialized. The repository stays, so the result is
/// partial and the record stays accepted; the same request completes it.
#[test]
fn a_create_interrupted_after_initialization_is_partial_and_its_retry_completes() {
    let mut desk = Desk::new();
    let site = Site::new();
    fs::create_dir(&site.root).unwrap();
    let (confirmation, preview) = desk.preview(&site.create());
    assert_eq!(effects(&preview)[0], PreviewEffect::RepositoryInitialized);
    desk.fail_once_at(FailurePoint::AfterRepositoryInitialization);

    let partial = desk.execute(REQUEST_1, &confirmation, site.create());

    assert_eq!(partial.outcome, Outcome::Partial, "{}", partial.message);
    assert_eq!(partial.code, ResultCode::InternalError);
    assert_eq!(partial.effects.commit_oid, None);
    assert_eq!(recovery_actions(&partial), ["request.retry"]);
    assert!(git2::Repository::open(&site.root).is_ok());
    assert_eq!(site.configuration(), None);
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    assert_eq!(desk.pending_journal_rows(), 1);
    // Its confirmation is its own meanwhile.
    assert_eq!(
        desk.execute(REQUEST_2, &confirmation, site.create()).code,
        ResultCode::ConfirmationUsed
    );

    desk.reopen();
    desk.clock.set(START + 3600);
    let finished = desk.execute(REQUEST_1, &confirmation, site.create());

    assert_eq!(
        assert_enabled(&finished, &site.root),
        site.head().to_string()
    );
    assert_eq!(initializations(&site.root), 1);
    assert_eq!(
        local_config(&site.root, "user.name").as_deref(),
        Some("Ada Example")
    );
    assert_eq!(desk.pending_journal_rows(), 0);
    assert!(desk.registered(&site.root));
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

/// The retry of a create whose first call never began observes again.
#[test]
fn a_retry_of_a_create_before_any_effect_observes_again() {
    let mut desk = Desk::new();
    let site = Site::new();
    let (confirmation, _) = desk.preview(&site.create());
    desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    desk.execute(REQUEST_1, &confirmation, site.create());
    fs::create_dir(&site.root).unwrap();
    fs::write(site.root.join("notes.txt"), "mine\n").unwrap();
    let before = desk.surroundings(site.parent.path());

    let refused = desk.execute(REQUEST_1, &confirmation, site.create());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert_eq!(desk.surroundings(site.parent.path()), before);
    desk.assert_nothing_left(REQUEST_1);
}

#[test]
fn repo_create_with_a_global_identity_says_it_will_be_written_and_writes_it() {
    let Some(home) = in_a_process_with_its_own_git_home(
        "repo_create_with_a_global_identity_says_it_will_be_written_and_writes_it",
    ) else {
        return;
    };
    let global = write_global_identity(&home);
    let desk = Desk::new();
    let site = Site::new();
    let create = || {
        Mutation::RepoCreate(RepoCreateInput {
            identity: None,
            ..site.input()
        })
    };

    let (confirmation, preview) = desk.preview(&create());

    let details = repository_preview(&preview);
    assert_eq!(details.identity.name, "Global Person");
    assert_eq!(details.identity.email, "global@example.invalid");
    assert_eq!(details.identity.source, PreviewIdentitySource::Global);
    assert!(details.identity.written_locally);
    assert!(effects(&preview).contains(&PreviewEffect::IdentityWritten));

    let created = desk.execute(REQUEST_1, &confirmation, create());

    assert_enabled(&created, &site.root);
    assert_eq!(
        local_config(&site.root, "user.name").as_deref(),
        Some("Global Person")
    );
    assert_eq!(
        local_config(&site.root, "user.email").as_deref(),
        Some("global@example.invalid")
    );
    let repository = git2::Repository::open(&site.root).unwrap();
    assert_eq!(
        repository.find_commit(site.head()).unwrap().author().name(),
        Some("Global Person")
    );
    assert_eq!(global_configuration(), Some(global));
}

/// The global identity changes between the preview and the request: what
/// was shown is not what would be written.
#[test]
fn a_global_identity_that_changes_after_the_preview_is_external_change() {
    let Some(home) = in_a_process_with_its_own_git_home(
        "a_global_identity_that_changes_after_the_preview_is_external_change",
    ) else {
        return;
    };
    write_global_identity(&home);
    let desk = Desk::new();
    let site = Site::new();
    let create = || {
        Mutation::RepoCreate(RepoCreateInput {
            identity: None,
            ..site.input()
        })
    };
    let (confirmation, _) = desk.preview(&create());
    fs::write(
        home.join(".gitconfig"),
        "[user]\n\tname = Someone Else\n\temail = global@example.invalid\n",
    )
    .unwrap();

    let refused = desk.execute(REQUEST_1, &confirmation, create());

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    assert!(!site.root.exists());
    desk.assert_nothing_left(REQUEST_1);
}

#[test]
fn repo_create_with_no_identity_at_all_is_identity_required_and_makes_no_directory() {
    let Some(_home) = in_a_process_with_its_own_git_home(
        "repo_create_with_no_identity_at_all_is_identity_required_and_makes_no_directory",
    ) else {
        return;
    };
    let desk = Desk::new();
    let site = Site::new();
    let create = Mutation::RepoCreate(RepoCreateInput {
        identity: None,
        ..site.input()
    });
    let before = desk.surroundings(site.parent.path());

    let refused = desk.prepare(&create);

    assert_eq!(refused.code, ResultCode::IdentityRequired);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.data, None);
    // There is no repository to set an identity on: the request is sent
    // again with one.
    assert!(refused.recovery.is_empty());
    assert!(!site.root.exists());
    assert_eq!(desk.confirmation_rows(), 0);
    assert_eq!(desk.surroundings(site.parent.path()), before);
    assert_eq!(global_configuration(), None);
}

/// Someone else enables the repository after the request is accepted and
/// before its own operation looks. The operation finds it enabled and
/// writes nothing: the request is a no-op, and the other's commit is not
/// reported as its own, on the first call or on a retry.
#[test]
fn an_enablement_made_by_another_is_not_reported_as_the_request_s() {
    for point in [None, Some(FailurePoint::BeforeRequestSettlement)] {
        let mut desk = Desk::new();
        let plain = Plain::new();
        let (confirmation, _) = desk.preview(&plain.enable());
        if let Some(point) = point {
            desk.fail_once_at(point);
        }
        let root = plain.root.clone();
        let data = desk.data.path().to_owned();
        desk.service.set_request_hook_for_testing(move || {
            let other = RepositoryService::open_at(&data).unwrap();
            other.enable(support::enable_request(&root)).unwrap();
        });

        let mut answered = desk.execute(REQUEST_1, &confirmation, plain.enable());
        if point.is_some() {
            desk.reopen();
            desk.clock.set(START + 3600);
            answered = desk.execute(REQUEST_1, &confirmation, plain.enable());
        }

        assert_eq!(answered.code, ResultCode::AlreadyApplied, "{point:?}");
        assert_eq!(answered.outcome, Outcome::Noop, "{point:?}");
        assert_eq!(answered.effects.commit_oid, None, "{point:?}");
        assert_eq!(answered.effects.checkpoint, CheckpointEffect::Unchanged);
        assert_eq!(plain.initializations(), 1, "{point:?}");
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished)
        );
    }
}

// ---------------------------------------------------------------------
// What a retry still compares once its operation has begun
// ---------------------------------------------------------------------

/// The identity a preview showed is what the request commits as, on
/// every call of it. A create that supplied none is interrupted after
/// its repository is initialized, and the global identity changes: the
/// retry is refused and writes nothing more. The request's own work
/// never changes the global configuration, so nothing excuses the
/// difference.
#[test]
fn a_retry_whose_effective_identity_changed_is_external_change() {
    let Some(home) = in_a_process_with_its_own_git_home(
        "a_retry_whose_effective_identity_changed_is_external_change",
    ) else {
        return;
    };
    let shown = write_global_identity(&home);
    let others: [(&str, Option<&str>); 2] = [
        (
            "another identity",
            Some("[user]\n\tname = Someone Else\n\temail = else@example.invalid\n"),
        ),
        ("no identity", None),
    ];
    for (what, other) in others {
        fs::write(home.join(".gitconfig"), &shown).unwrap();
        let mut desk = Desk::new();
        let site = Site::new();
        fs::create_dir(&site.root).unwrap();
        let create = || {
            Mutation::RepoCreate(RepoCreateInput {
                identity: None,
                ..site.input()
            })
        };
        let (confirmation, _) = desk.preview(&create());
        desk.fail_once_at(FailurePoint::AfterRepositoryInitialization);
        let partial = desk.execute(REQUEST_1, &confirmation, create());
        assert_eq!(partial.outcome, Outcome::Partial, "{what}");
        match other {
            Some(other) => fs::write(home.join(".gitconfig"), other).unwrap(),
            None => fs::remove_file(home.join(".gitconfig")).unwrap(),
        }
        let before = tree(&site.root);

        desk.reopen();
        desk.clock.set(START + 3600);
        let refused = desk.execute(REQUEST_1, &confirmation, create());

        assert_eq!(refused.code, ResultCode::ExternalChange, "{what}");
        assert_eq!(refused.effects.commit_oid, None, "{what}");
        // Nothing more is written: no identity, no configuration, no
        // commit.
        assert_eq!(tree(&site.root), before, "{what}");
        assert_eq!(local_config(&site.root, "user.name"), None, "{what}");
        assert_eq!(site.configuration(), None, "{what}");
        // The request is still there, with its operation in the way of
        // any other: a new preview could not be used.
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted),
            "{what}"
        );
        assert_offered_its_own_abandon(&refused, what);

        // With the identity that was shown back, the same request
        // completes as it.
        fs::write(home.join(".gitconfig"), &shown).unwrap();
        let finished = desk.execute(REQUEST_1, &confirmation, create());
        assert_enabled(&finished, &site.root);
        assert_eq!(
            local_config(&site.root, "user.name").as_deref(),
            Some("Global Person"),
            "{what}"
        );
        let repository = git2::Repository::open(&site.root).unwrap();
        assert_eq!(
            repository.find_commit(site.head()).unwrap().author().name(),
            Some("Global Person"),
            "{what}"
        );
        assert_eq!(initializations(&site.root), 1, "{what}");
    }
}

/// The same for `repo enable`, which finds its identity itself, while
/// anything that depends on the identity is still to do: the enablement
/// is killed after it wrote the configuration and before it committed,
/// and the repository's own identity is replaced. The retry would commit
/// as an identity nobody was shown, and is refused.
#[test]
fn a_retry_of_repo_enable_before_its_commit_compares_the_identity_it_showed() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    killed_at(
        &desk,
        &plain.root,
        true,
        &confirmation,
        FailurePoint::BeforeInitializationCommit,
    );
    assert_eq!(
        desk.count(
            "SELECT COUNT(*) FROM operation_records
              WHERE state != 'completed' AND completed_step = 'configuration_written'"
        ),
        1
    );
    let mut config = git2::Config::open(&plain.root.join(".git/config")).unwrap();
    config.set_str("user.name", "Someone Else").unwrap();

    desk.reopen();
    let refused = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_offered_its_own_abandon(&refused, "the retry");
    assert_eq!(plain.initializations(), 0);
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
}

/// Once the initialization commit is recorded, nothing the request has
/// left to do reads the identity: it is written and the commit is
/// authored. A request answered `registration_pending` was told to send
/// itself again, and a change of identity since is none of its business:
/// the retry registers the repository and reports the same commit.
#[test]
fn a_retry_after_the_recorded_commit_does_not_compare_the_identity() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRegistryWrite);
    let pending = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(pending.code, ResultCode::RegistrationPending);
    assert_eq!(recovery_actions(&pending), ["request.retry"]);
    let commit = plain.head();
    let mut config = git2::Config::open(&plain.root.join(".git/config")).unwrap();
    config.set_str("user.name", "Someone Else").unwrap();

    desk.reopen();
    desk.clock.set(START + 3600);
    let finished = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(assert_enabled(&finished, &plain.root), commit.to_string());
    assert!(desk.registered(&plain.root));
    assert_eq!(plain.initializations(), 1);
    let authored = plain.fixture.repository.find_commit(commit).unwrap();
    assert_eq!(authored.author().name(), Some("Manyhands Test"));
}

/// The allowance for a create's own directory is for a create whose
/// operation has begun. With no row, no attempt of the request reached
/// the domain, so an empty directory where the preview saw none was
/// made by someone else: the retry observes again and is refused.
#[test]
fn an_empty_directory_made_by_another_before_the_create_began_is_external_change() {
    let mut desk = Desk::new();
    let site = Site::new();
    let (confirmation, _) = desk.preview(&site.create());
    desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    desk.execute(REQUEST_1, &confirmation, site.create());
    assert_eq!(desk.count("SELECT COUNT(*) FROM operation_records"), 0);
    fs::create_dir(&site.root).unwrap();

    let refused = desk.execute(REQUEST_1, &confirmation, site.create());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert!(fs::read_dir(&site.root).unwrap().next().is_none());
    desk.assert_nothing_left(REQUEST_1);
}

const KILLED_PARENT: &str = "MANYHANDS_KILLED_PARENT";
const KILLED_DATA: &str = "MANYHANDS_KILLED_DATA";
const KILLED_CONFIRMATION: &str = "MANYHANDS_KILLED_CONFIRMATION";
const KILLED_POINT: &str = "MANYHANDS_KILLED_POINT";
const KILLED_EXISTING: &str = "MANYHANDS_KILLED_EXISTING";

/// The points a child process can be made to end at.
const KILL_POINTS: [FailurePoint; 3] = [
    FailurePoint::BeforeRepositoryInitialization,
    FailurePoint::BeforeInitializationCommit,
    FailurePoint::AfterInitializationCommit,
];

/// The create a killed child makes, and its parent retries.
fn killed_create(parent: &Path) -> Mutation {
    Mutation::RepoCreate(RepoCreateInput {
        root: parent.join("new"),
        primary_branch: "main".to_owned(),
        identity: Some(identity()),
        slug_prefix: None,
    })
}

/// The enable a killed child makes, and its parent retries.
fn killed_enable(root: &Path) -> Mutation {
    Mutation::RepoEnable(RepoEnableInput {
        root: root.to_owned(),
        primary_branch: "main".to_owned(),
        identity: None,
        slug_prefix: None,
    })
}

/// The process `killed_at` starts: it submits the request and its
/// process ends at the fault point, inside the domain operation, with
/// nothing after it run: no rollback, and no settlement.
#[test]
fn killed_at_fault_point_child() {
    let Some(parent) = std::env::var_os(KILLED_PARENT) else {
        return;
    };
    let name = std::env::var(KILLED_POINT).unwrap();
    let point = KILL_POINTS
        .into_iter()
        .find(|point| format!("{point:?}") == name)
        .expect("a known fault point");
    let service = RepositoryService::open_at_with_exit_point_for_testing(
        Path::new(&std::env::var_os(KILLED_DATA).unwrap()),
        point,
    )
    .unwrap()
    .with_clock_for_testing(TestClock::at(START));
    let mutation = if std::env::var_os(KILLED_EXISTING).is_some() {
        killed_enable(Path::new(&parent))
    } else {
        killed_create(Path::new(&parent))
    };
    execute_confirmed(
        &service,
        REQUEST_1,
        &std::env::var(KILLED_CONFIRMATION).unwrap(),
        mutation,
    );
    // Not reached: the process ended at the fault point.
}

/// Runs `REQUEST_1` in a child process that ends at `point`: a create
/// of `new` under `directory`, or with `existing` an enable of it.
fn killed_at(
    desk: &Desk,
    directory: &Path,
    existing: bool,
    confirmation: &str,
    point: FailurePoint,
) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "killed_at_fault_point_child", "--nocapture"])
        .env(KILLED_PARENT, directory)
        .env(KILLED_DATA, desk.data.path())
        .env(KILLED_CONFIRMATION, confirmation)
        .env(KILLED_POINT, format!("{point:?}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if existing {
        command.env(KILLED_EXISTING, "1");
    }
    assert_eq!(
        command.status().unwrap().code(),
        Some(manyhands::repository::FAILURE_POINT_EXIT_STATUS),
        "{point:?}: the child ended at the fault point"
    );
}

/// A create is killed after it made its directory and before it
/// recorded a step. The preview saw no target, and the retry sees an
/// empty directory: the very thing the preview said it would make, under
/// the request's own pending operation. The retry completes.
#[test]
fn a_create_killed_after_it_made_its_directory_is_completed_by_its_retry() {
    let mut desk = Desk::new();
    let site = Site::new();
    let create = || killed_create(site.parent.path());
    let (confirmation, _) = desk.preview(&create());
    killed_at(
        &desk,
        site.parent.path(),
        false,
        &confirmation,
        FailurePoint::BeforeRepositoryInitialization,
    );
    // What the dead process left: its directory, empty; its record
    // accepted; its operation pending, with no step.
    assert_eq!(tree(site.parent.path()).len(), 1);
    assert!(site.root.is_dir());
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    assert_eq!(desk.pending_journal_rows(), 1);
    assert_eq!(
        desk.count("SELECT COUNT(*) FROM operation_records WHERE completed_step IS NOT NULL"),
        0
    );

    desk.reopen();
    desk.clock.set(START + 3600);
    let finished = desk.execute(REQUEST_1, &confirmation, create());

    assert_enabled(&finished, &site.root);
    assert_eq!(initializations(&site.root), 1);
    assert_eq!(desk.pending_journal_rows(), 0);
    assert!(desk.registered(&site.root));
}

/// That allowance is for the request's own directory and nothing else:
/// a file in it is still a change from elsewhere.
#[test]
fn a_create_killed_after_it_made_its_directory_still_refuses_what_is_put_in_it() {
    let mut desk = Desk::new();
    let site = Site::new();
    let create = || killed_create(site.parent.path());
    let (confirmation, _) = desk.preview(&create());
    killed_at(
        &desk,
        site.parent.path(),
        false,
        &confirmation,
        FailurePoint::BeforeRepositoryInitialization,
    );
    fs::write(site.root.join("notes.txt"), "mine\n").unwrap();

    desk.reopen();
    let refused = desk.execute(REQUEST_1, &confirmation, create());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_offered_its_own_abandon(&refused, "the retry");
    assert!(git2::Repository::open(&site.root).is_err());
}

/// Two requests present one confirmation at once. Each finds it free
/// when it looks; the write that accepts it is conditional, so one of
/// them gets it and the other is told it is used.
#[test]
fn of_two_requests_racing_for_one_confirmation_one_is_confirmation_used() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let (data, clock, root) = (
        desk.data.path().to_owned(),
        desk.clock.clone(),
        plain.root.clone(),
    );
    let presented = confirmation.clone();
    // After the first request has found the confirmation its own to use
    // and before it is written, the second is accepted with it and runs.
    desk.service.set_acceptance_hook_for_testing(move || {
        let other = mutation::service_with_clock(&data, &clock);
        let enabled = execute_confirmed(
            &other,
            REQUEST_2,
            &presented,
            Mutation::RepoEnable(RepoEnableInput {
                root,
                primary_branch: "main".to_owned(),
                identity: None,
                slug_prefix: None,
            }),
        );
        assert_eq!(enabled.code, ResultCode::Ok, "{}", enabled.message);
    });

    let lost = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_refused(&lost, ResultCode::ConfirmationUsed, &[]);
    assert_eq!(desk.record(REQUEST_1), None);
    assert_eq!(
        desk.record(REQUEST_2).map(|record| record.state),
        Some(RequestState::Finished)
    );
    assert_eq!(desk.accepted_confirmations(), 1);
    assert_eq!(plain.initializations(), 1);
}

// ---------------------------------------------------------------------
// Which commit is the request's, and what a row that has ended says
// ---------------------------------------------------------------------

/// The reason a completed row with no checkpoint step says nothing for
/// these commands. An enablement is cut off after its commit and before
/// the step that records it. The call that takes its row up finds the
/// repository enabled and completes the row, which still holds the step
/// before the commit; that call's result is lost as well. The next call
/// meets a completed row with no checkpoint step, of an operation that
/// committed: it reports that commit and makes no second one, and does
/// not ask for the confirmation again.
#[test]
fn a_row_that_completed_without_its_checkpoint_step_still_reports_its_commit() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::AfterInitializationCommit);

    let cut_off = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(cut_off.outcome, Outcome::Partial, "{}", cut_off.message);
    let commit = plain.head();
    assert_eq!(cut_off.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(plain.initializations(), 1);
    assert_eq!(
        desk.count(
            "SELECT COUNT(*) FROM operation_records
              WHERE state != 'completed' AND completed_step = 'configuration_written'"
        ),
        1
    );

    desk.fail_once_at(FailurePoint::BeforeRequestSettlement);
    desk.execute(REQUEST_1, &confirmation, plain.enable());
    // The row has completed, and holds no checkpoint step.
    assert_eq!(
        desk.count(
            "SELECT COUNT(*) FROM operation_records
              WHERE state = 'completed' AND completed_step = 'configuration_written'"
        ),
        1
    );
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    desk.reopen();
    desk.clock.set(START + 3600);
    let finished = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(assert_enabled(&finished, &plain.root), commit.to_string());
    assert_eq!(plain.head(), commit);
    assert_eq!(plain.initializations(), 1);
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

/// An enablement is killed after it wrote the configuration and before
/// it committed it, and someone commits the file by hand. The row shows
/// the operation wrote the file; the commit that holds it is not the
/// operation's, and is not reported as the request's.
#[test]
fn a_configuration_committed_by_hand_is_not_reported_as_the_request_s_commit() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    killed_at(
        &desk,
        &plain.root,
        true,
        &confirmation,
        FailurePoint::BeforeInitializationCommit,
    );
    let written = plain
        .configuration()
        .expect("the configuration was written");
    assert_eq!(plain.initializations(), 0);
    let by_hand = World::commit_in(&plain.root, canonical::CONFIG_PATH, &written);

    desk.reopen();
    let answered = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(
        answered.code,
        ResultCode::AlreadyApplied,
        "{}",
        answered.message
    );
    assert_eq!(answered.outcome, Outcome::Noop);
    assert_eq!(answered.effects.commit_oid, None);
    assert_eq!(plain.head(), by_hand);
    assert_eq!(plain.initializations(), 0);
    assert_eq!(desk.pending_journal_rows(), 0);
}

/// An enablement ends having done nothing, for want of an identity, and
/// its answer is lost. By the retry the identity is back and nothing has
/// changed. The retry does not say that something has: the request did
/// not run, its record is gone, and the same request is what is sent.
#[test]
fn a_lost_refusal_whose_cause_has_gone_is_not_called_a_change() {
    let Some(_home) = in_a_process_with_its_own_git_home(
        "a_lost_refusal_whose_cause_has_gone_is_not_called_a_change",
    ) else {
        return;
    };
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let config_path = plain.root.join(".git/config");
    let with_identity = fs::read(&config_path).unwrap();
    let path = config_path.clone();
    desk.service.set_request_hook_for_testing(move || {
        let mut config = git2::Config::open(&path).unwrap();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
    });
    desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(plain.configuration(), None);
    assert_eq!(
        desk.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    fs::write(&config_path, with_identity).unwrap();

    desk.reopen();
    let answered = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(answered.code, ResultCode::Busy, "{}", answered.message);
    assert_eq!(recovery_actions(&answered), ["request.retry"]);
    desk.assert_nothing_left(REQUEST_1);
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
}

/// The same lost refusal while its cause stands is answered with the
/// cause.
#[test]
fn a_lost_refusal_whose_cause_stands_is_answered_with_the_cause() {
    let Some(_home) = in_a_process_with_its_own_git_home(
        "a_lost_refusal_whose_cause_stands_is_answered_with_the_cause",
    ) else {
        return;
    };
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let path = plain.root.join(".git/config");
    desk.service.set_request_hook_for_testing(move || {
        let mut config = git2::Config::open(&path).unwrap();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
    });
    desk.execute(REQUEST_1, &confirmation, plain.enable());

    desk.reopen();
    let answered = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_eq!(answered.code, ResultCode::IdentityRequired);
    assert_eq!(recovery_actions(&answered), ["repo.identity_set"]);
    desk.assert_nothing_left(REQUEST_1);
}

/// The preview is what is consented to, so it names everything the
/// command does to the repository: here also that HEAD, which names a
/// branch with no commit that is not the primary, is moved onto the
/// primary, and that Git's own exclude file is written.
#[test]
fn a_preview_of_an_unborn_repository_says_that_head_is_moved() {
    let desk = Desk::new();
    let fixture = support::unborn_repository();
    {
        let mut config = fixture.repository.config().unwrap();
        config.set_str("user.name", "Manyhands Test").unwrap();
        config
            .set_str("user.email", "manyhands-test@example.invalid")
            .unwrap();
    }
    fixture.repository.set_head("refs/heads/elsewhere").unwrap();
    let plain = Plain {
        root: fixture.root.canonicalize().unwrap(),
        fixture,
    };
    let exclude = plain.root.join(".git/info/exclude");
    let excluded = fs::read(&exclude).ok();

    let (confirmation, preview) = desk.preview(&plain.enable());

    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::HeadMoved,
            PreviewEffect::WorktreeExclusionWritten,
            PreviewEffect::ConfigurationWritten,
            PreviewEffect::InitializationCommitted,
            PreviewEffect::RepositoryRegistered,
        ]
    );
    // The preview did neither.
    assert_eq!(fs::read(&exclude).ok(), excluded);
    assert_eq!(
        plain
            .fixture
            .repository
            .find_reference("HEAD")
            .unwrap()
            .symbolic_target(),
        Some("refs/heads/elsewhere")
    );

    let enabled = desk.execute(REQUEST_1, &confirmation, plain.enable());

    assert_enabled(&enabled, &plain.root);
    assert_eq!(
        plain
            .fixture
            .repository
            .find_reference("HEAD")
            .unwrap()
            .symbolic_target(),
        Some("refs/heads/main")
    );
    assert!(
        String::from_utf8(fs::read(&exclude).unwrap())
            .unwrap()
            .contains(".manyhands/worktrees/")
    );
}

/// On the primary branch, born or not, HEAD stays where it is, and the
/// preview does not say otherwise.
#[test]
fn a_preview_on_the_primary_branch_moves_no_head() {
    let desk = Desk::new();
    let plain = Plain::new();
    let (_, preview) = desk.preview(&plain.enable());
    assert!(!effects(&preview).contains(&PreviewEffect::HeadMoved));
    assert!(effects(&preview).contains(&PreviewEffect::WorktreeExclusionWritten));
}

/// The secrets are put where these commands read: in the configuration
/// file, which each parses, and in the location of the remote the
/// configuration names for publication, which enablement looks up. Each
/// command is then made to fail on what it read, in `prepare` and under
/// the lease in `execute`, and nothing it returns or records holds any
/// of it. No passphrase is planted: neither command is given one or
/// reads a key.
#[test]
fn no_failure_of_these_commands_carries_what_they_read() {
    let sentinels = [URL_SENTINEL, BODY_SENTINEL];
    // A configuration that does not parse, and one that names a remote
    // whose location holds a password.
    let unreadable = format!("primary_branch = \"{BODY_SENTINEL}\"\nnot = [toml {BODY_SENTINEL}");
    let naming_a_remote =
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
    let plant = |root: &Path, configuration: &str| {
        let repository = git2::Repository::open(root).unwrap();
        if repository.find_remote("origin").is_err() {
            repository
                .remote(
                    "origin",
                    &format!("https://deploy:{URL_SENTINEL}@example.invalid/repository.git"),
                )
                .unwrap();
        }
        fs::create_dir_all(root.join(".manyhands")).unwrap();
        fs::write(root.join(canonical::CONFIG_PATH), configuration).unwrap();
        // Out of the way of the clean-worktree check, so that the
        // configuration itself is what is read.
        fs::write(root.join(".git/info/exclude"), ".manyhands/\n").unwrap();
    };
    let scan = |desk: &Desk, what: &str, value: serde_json::Value| {
        assert_eq!(golden::find_sentinel(&value, &sentinels), None, "{what}");
        mutation::assert_request_stores_exclude(desk.data.path(), &sentinels);
    };

    // `prepare`, on a configuration it cannot read.
    let desk = Desk::new();
    let plain = Plain::new();
    plant(&plain.root, &unreadable);
    let refused = desk.prepare(&plain.enable());
    assert_eq!(refused.code, ResultCode::InvalidConfiguration);
    scan(&desk, "prepare", serde_json::to_value(&refused).unwrap());

    // `execute`: the same file, and then the remote, appear after the
    // request is accepted, and the domain operation reads them.
    for (what, configuration, code) in [
        (
            "an unreadable configuration",
            unreadable.as_str(),
            ResultCode::InvalidConfiguration,
        ),
        (
            "a publication remote that cannot be used",
            naming_a_remote,
            ResultCode::InvalidRemote,
        ),
    ] {
        let desk = Desk::new();
        let plain = Plain::new();
        let (confirmation, _) = desk.preview(&plain.enable());
        let (root, planted) = (plain.root.clone(), configuration.to_owned());
        desk.service
            .set_request_hook_for_testing(move || plant(&root, &planted));

        let failed = desk.execute(REQUEST_1, &confirmation, plain.enable());

        assert_eq!(failed.code, code, "{what}: {}", failed.message);
        scan(&desk, what, serde_json::to_value(&failed).unwrap());
        // The scan read the rows the request left: its confirmation,
        // and the row of the operation that was refused.
        assert_eq!(desk.confirmation_rows(), 1, "{what}");
        assert_eq!(
            desk.count("SELECT COUNT(*) FROM operation_records"),
            1,
            "{what}"
        );
        desk.assert_nothing_left(REQUEST_1);
    }
}

// ---------------------------------------------------------------------
// The confirmed commands of administration
// ---------------------------------------------------------------------

/// An enabled, registered repository with committed, indexed items, and
/// a service over it that is timed by a clock the test sets.
struct Home {
    world: World,
    clock: Arc<TestClock>,
}

impl Home {
    fn new() -> Self {
        let clock = TestClock::at(START);
        let world = World::with_service(|data| mutation::service_with_clock(data, &clock));
        Self { world, clock }
    }

    /// Another service over the same data, as a new process would open.
    fn reopen(&mut self) {
        self.world.service = mutation::service_with_clock(self.world.data.path(), &self.clock);
    }

    /// Another service over the same data that fails once at `point`.
    fn fail_once_at(&mut self, point: FailurePoint) {
        self.world.service = RepositoryService::open_at_with_failure_point_for_testing(
            self.world.data.path(),
            point,
        )
        .unwrap()
        .with_clock_for_testing(self.clock.clone());
    }

    fn service(&self) -> &RepositoryService {
        &self.world.service
    }

    fn root(&self) -> &Path {
        &self.world.root
    }

    fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        prepare_with(self.service(), mutation)
    }

    /// A preview that is given, and the confirmation it gives.
    fn preview(&self, mutation: &Mutation) -> (String, PreviewDto) {
        let envelope = self.prepare(mutation);
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    fn execute(
        &self,
        request_id: &str,
        confirmation: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        execute_confirmed(self.service(), request_id, confirmation, mutation)
    }

    /// The same request again, presenting nothing: its record holds its
    /// consent.
    fn retry(&self, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        execute_with(self.service(), request_id, mutation)
    }

    fn state(&self, request_id: &str) -> Option<RequestState> {
        self.world.record(request_id).map(|record| record.state)
    }

    fn count(&self, query: &str) -> i64 {
        items::index(self.world.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    fn confirmation_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM confirmation_records")
    }

    /// Nothing is left of a request that was rejected: no record, no
    /// confirmation held, and no operation in flight.
    fn assert_nothing_left(&self, request_id: &str) {
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

    fn git(&self) -> git2::Repository {
        git2::Repository::open(self.root()).unwrap()
    }

    /// Adds a remote by hand, as `git remote add` would.
    fn add_remote(&self, name: &str, url: &str) {
        self.git().remote(name, url).unwrap();
    }

    fn remote_url(&self, name: &str) -> Option<String> {
        self.git()
            .find_remote(name)
            .ok()
            .and_then(|remote| remote.url().map(str::to_owned))
    }

    fn remove_remote(&self, name: &str) -> Mutation {
        Mutation::RemoteRemove(RemoteRemoveInput {
            root: self.root().to_owned(),
            name: name.to_owned(),
        })
    }

    /// Selects the publication remote, or clears the selection, as a
    /// request of its own would.
    fn select(&self, request_id: &str, name: Option<&str>) {
        select_through(self.service(), request_id, self.root(), name);
    }
}

fn select_through(service: &RepositoryService, request_id: &str, root: &Path, name: Option<&str>) {
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

/// A location for a remote that can be published to.
const BACKUP: &str = "git@example.invalid:team/backup.git";

fn remote_removal(preview: &PreviewDto) -> &RemoteRemovalPreviewDto {
    match &preview.details {
        PreviewDetailsDto::RemoteRemoval(details) => details,
        other => panic!("not a preview of a remote's removal: {other:?}"),
    }
}

/// A request of administration that did what it was asked: a success
/// that no effect shows.
fn assert_done(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(envelope.recovery.is_empty());
}

/// The same for a request whose end state held already.
fn assert_noop(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        envelope.code,
        ResultCode::AlreadyApplied,
        "{}",
        envelope.message
    );
    assert_eq!(envelope.outcome, Outcome::Noop);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(envelope.recovery.is_empty());
}

// ---------------------------------------------------------------------
// `remote remove`
// ---------------------------------------------------------------------

#[test]
fn a_preview_of_remote_remove_names_the_remote_and_says_whether_polling_is_affected() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    let before = tree(home.root());

    let (_, preview) = home.preview(&home.remove_remote("backup"));

    assert_eq!(tree(home.root()), before);
    assert_eq!(home.confirmation_rows(), 1);
    assert_eq!(home.world.request_rows(), 0);
    assert_eq!(preview.command, "remote remove");
    assert_eq!(preview.scope.remote.as_deref(), Some("backup"));
    assert_eq!(
        preview.scope.repository.as_deref(),
        Some(home.root().to_str().unwrap())
    );
    assert_eq!(effects(&preview), [PreviewEffect::RemoteRemoved]);
    let details = remote_removal(&preview);
    assert_eq!(details.root, home.root().to_str().unwrap());
    assert_eq!(details.name, "backup");
    assert_eq!(details.fetch_location.as_deref(), Some(BACKUP));
    assert_eq!(details.push_location.as_deref(), Some(BACKUP));
    assert!(!details.publication_affected);
    assert!(!details.polling_affected);

    // The repository's polling state still names the remote: the index
    // has not caught up with a selection that was cleared.
    let named = items::index(home.world.data.path())
        .execute(
            "UPDATE remote_polling_state SET remote_name='backup', primary_branch='main'",
            [],
        )
        .unwrap();
    assert_eq!(named, 1, "the registration has a polling row");
    let (_, preview) = home.preview(&home.remove_remote("backup"));
    assert!(remote_removal(&preview).polling_affected);
    assert!(!remote_removal(&preview).publication_affected);
}

/// Git removes a remote's tracking references and the upstream of every
/// branch that follows it. The preview says so when there are any, and
/// they are what the removal removes.
#[test]
fn a_preview_of_remote_remove_lists_what_git_removes_with_the_remote() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference("refs/remotes/backup/main", head, false, "test")
        .unwrap();
    let mut config = repository.config().unwrap();
    config.set_str("branch.main.remote", "backup").unwrap();
    config
        .set_str("branch.main.merge", "refs/heads/main")
        .unwrap();
    let files = tree(home.root())
        .into_iter()
        .filter(|(path, _)| !path.starts_with(".git"))
        .collect::<Vec<_>>();

    let (confirmation, preview) = home.preview(&home.remove_remote("backup"));

    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::RemoteRemoved,
            PreviewEffect::RemoteTrackingRefsRemoved,
            PreviewEffect::BranchUpstreamsRemoved,
        ]
    );

    let removed = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_done(&removed);
    let repository = home.git();
    assert!(repository.find_remote("backup").is_err());
    assert!(
        repository
            .find_reference("refs/remotes/backup/main")
            .is_err()
    );
    assert!(
        repository
            .config()
            .unwrap()
            .get_string("branch.main.remote")
            .is_err()
    );
    // Nothing else: the branch, its commit and every file are as they
    // were.
    assert_eq!(
        repository.head().unwrap().peel_to_commit().unwrap().id(),
        head
    );
    assert_eq!(
        tree(home.root())
            .into_iter()
            .filter(|(path, _)| !path.starts_with(".git"))
            .collect::<Vec<_>>(),
        files
    );
}

/// A tracking reference that appears after the preview is something the
/// removal would remove and the preview did not name.
#[test]
fn what_git_would_remove_and_the_preview_did_not_name_is_external_change() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, preview) = home.preview(&home.remove_remote("backup"));
    assert_eq!(effects(&preview), [PreviewEffect::RemoteRemoved]);
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference("refs/remotes/backup/main", head, false, "test")
        .unwrap();

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    home.assert_nothing_left(REQUEST_1);
    assert!(
        repository
            .find_reference("refs/remotes/backup/main")
            .is_ok()
    );
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
}

#[test]
fn prepare_for_the_publication_remote_is_publication_remote_in_use_with_no_confirmation() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    home.select(REQUEST_3, Some("backup"));

    let refused = home.prepare(&home.remove_remote("backup"));

    assert_eq!(refused.code, ResultCode::PublicationRemoteInUse);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.data, None);
    assert!(refused.recovery.is_empty());
    assert_eq!(home.confirmation_rows(), 0);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
}

#[test]
fn prepare_for_a_remote_the_repository_does_not_have_is_a_no_op_with_no_confirmation() {
    let home = Home::new();

    let noop = home.prepare(&home.remove_remote("backup"));
    assert_eq!(noop.code, ResultCode::AlreadyApplied);
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_eq!(noop.data, None);

    let invalid = home.prepare(&home.remove_remote("not a name"));
    assert_eq!(invalid.code, ResultCode::InvalidRemote);
    assert_eq!(invalid.data, None);
    assert_eq!(home.confirmation_rows(), 0);
}

#[test]
fn selecting_the_remote_between_preview_and_confirmation_is_external_change() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.select(REQUEST_3, Some("backup"));

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    assert_eq!(refused.outcome, Outcome::Blocked);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
    // The new preview says what is in the way.
    let prepared = home.prepare(&home.remove_remote("backup"));
    assert_eq!(prepared.code, ResultCode::PublicationRemoteInUse);
}

#[test]
fn a_confirmed_remote_removal_shows_no_effect_and_is_replayed_and_not_reused() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    home.add_remote("other", "git@example.invalid:team/other.git");
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));

    let required = home.retry(REQUEST_1, home.remove_remote("backup"));
    assert_refused(
        &required,
        ResultCode::ConfirmationRequired,
        &["request.prepare"],
    );
    let other = home.execute(REQUEST_1, &confirmation, home.remove_remote("other"));
    assert_refused(&other, ResultCode::ConfirmationMismatch, &[]);

    let removed = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_done(&removed);
    let data = mutation::remote_data(&removed);
    assert_eq!(data.name.as_deref(), Some("backup"));
    assert_eq!(data.location, None);
    assert_eq!(home.remote_url("backup"), None);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(home.world.pending_journal_rows(), 0);

    // At any later time, whatever the retry presents and whatever has
    // been done since: the same result, and nothing runs.
    home.clock.set(START + 86_400);
    home.add_remote("backup", BACKUP);
    assert_eq!(
        home.execute(REQUEST_1, &confirmation, home.remove_remote("backup")),
        removed
    );
    assert_eq!(home.retry(REQUEST_1, home.remove_remote("backup")), removed);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));

    // The same request ID with another remote is another request.
    let changed = home.retry(REQUEST_1, home.remove_remote("other"));
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(changed.effects, Effects::not_requested());
    assert!(home.remote_url("other").is_some());
    // And the confirmation is used.
    let reused = home.execute(REQUEST_2, &confirmation, home.remove_remote("backup"));
    assert_refused(&reused, ResultCode::ConfirmationUsed, &[]);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
}

/// The remote is deleted and its step recorded, and the call fails
/// before its operation ends. An hour later, with the state the preview
/// covered changed by more than the request's own work, the same request
/// completes: nothing is observed again.
#[test]
fn a_removal_interrupted_after_the_remote_is_deleted_is_partial_and_its_retry_completes() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::AfterRemoteMutation);

    let partial = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_eq!(partial.code, ResultCode::InternalError);
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_eq!(partial.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&partial), ["request.retry"]);
    assert_eq!(home.remote_url("backup"), None);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(home.world.pending_journal_rows(), 1);

    home.reopen();
    home.clock.set(START + 3600);
    items::index(home.world.data.path())
        .execute("UPDATE remote_polling_state SET interval_seconds=900", [])
        .unwrap();
    let finished = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_done(&finished);
    assert_eq!(partial.operation_id, finished.operation_id);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(home.world.pending_journal_rows(), 0);
    assert_eq!(
        home.execute(REQUEST_1, &confirmation, home.remove_remote("backup")),
        finished
    );
}

/// The removal completes and its result is lost. Whatever has been done
/// since, a remote of the same name added among it, the retry is given
/// the request's result and is not refused.
#[test]
fn a_remote_removal_whose_result_was_lost_is_answered_with_its_result() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let lost = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(home.remote_url("backup"), None);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(home.world.pending_journal_rows(), 0);

    home.reopen();
    home.clock.set(START + 3600);
    home.add_remote("backup", "git@example.invalid:team/another.git");
    let answered = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_done(&answered);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    // Nothing ran: the remote added since is there.
    assert!(home.remote_url("backup").is_some());
}

/// A request is accepted and stops before its first effect. The remote
/// is selected for publication. The retry observes again and is refused:
/// the consent was to what was shown.
#[test]
fn a_retry_of_a_remote_removal_before_any_effect_observes_again() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    let died = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    home.select(REQUEST_3, Some("backup"));

    let refused = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
    home.assert_nothing_left(REQUEST_1);
}

/// The same interruption with nothing changed: the retry is honored,
/// also after the ten minutes.
#[test]
fn a_retry_of_a_remote_removal_before_any_effect_runs_when_nothing_has_changed() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    home.clock.set(START + 3600);

    let finished = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_done(&finished);
    assert_eq!(home.remote_url("backup"), None);
}

/// The remote is selected for publication after the request is accepted,
/// so the domain refuses the removal, and that answer is lost. The
/// domain closed the row it began, so the retry finds no operation of
/// its own: it observes again, and is told that what was previewed is
/// not what is there. Nothing is left of the request, and a new preview
/// says what stands in the way. With the selection cleared again nothing
/// has changed, and the same request and confirmation remove the remote.
#[test]
fn a_lost_refusal_of_a_remote_removal_is_answered_with_what_is_true() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let (data, clock, root) = (
        home.world.data.path().to_owned(),
        home.clock.clone(),
        home.root().to_owned(),
    );
    home.service().set_request_hook_for_testing(move || {
        let other = mutation::service_with_clock(&data, &clock);
        select_through(&other, REQUEST_3, &root, Some("backup"));
    });
    home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));

    home.reopen();
    let answered = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_eq!(answered.code, ResultCode::ExternalChange);
    assert_eq!(answered.outcome, Outcome::Blocked);
    assert_eq!(recovery_actions(&answered), ["request.prepare"]);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
    assert_eq!(
        home.prepare(&home.remove_remote("backup")).code,
        ResultCode::PublicationRemoteInUse
    );

    home.select(REQUEST_2, None);
    let removed = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_done(&removed);
    assert_eq!(home.remote_url("backup"), None);
}

/// A remote whose location holds a password: it is in no preview, no
/// confirmation, request or journal row and no envelope, on the path
/// that fails as on the one that completes.
#[test]
fn nothing_of_a_remote_removal_holds_the_remote_s_password() {
    let mut home = Home::new();
    home.add_remote(
        "backup",
        &format!("ssh://git:{URL_SENTINEL}@example.invalid/repository.git"),
    );
    let sentinels = [URL_SENTINEL];
    let scan = |home: &Home, what: &str, value: serde_json::Value| {
        assert_eq!(golden::find_sentinel(&value, &sentinels), None, "{what}");
        mutation::assert_request_stores_exclude(home.world.data.path(), &sentinels);
    };

    let prepared = home.prepare(&home.remove_remote("backup"));
    scan(&home, "prepare", serde_json::to_value(&prepared).unwrap());
    let preview = prepared.data.unwrap();
    assert_eq!(
        remote_removal(&preview).fetch_location.as_deref(),
        Some("ssh://git@example.invalid/repository.git")
    );
    let confirmation = preview.confirmation_id;

    home.fail_once_at(FailurePoint::AfterRemoteMutation);
    let partial = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_eq!(partial.outcome, Outcome::Partial);
    scan(&home, "a failure", serde_json::to_value(&partial).unwrap());
    // The scan read the rows the request left: its confirmation, its
    // record and its operation's row.
    assert_eq!(home.confirmation_rows(), 1);
    assert_eq!(home.world.request_rows(), 1);
    assert_eq!(home.world.pending_journal_rows(), 1);
    assert!(mutation::find_stored_sentinel(home.world.data.path(), &[&confirmation]).is_some());

    home.reopen();
    let finished = home.retry(REQUEST_1, home.remove_remote("backup"));
    assert_done(&finished);
    scan(&home, "the retry", serde_json::to_value(&finished).unwrap());
}

/// A request is accepted and stops before its first effect, and someone
/// else removes the remote. The request's end state holds: the retry is
/// a no-op, and is not refused for what has changed.
#[test]
fn a_retry_of_a_remote_removal_another_has_done_is_a_no_op() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    home.git().remote_delete("backup").unwrap();

    let answered = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_noop(&answered);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(home.world.pending_journal_rows(), 0);
}

// ---------------------------------------------------------------------
// `repo remove`
// ---------------------------------------------------------------------

impl Home {
    fn remove_repository(&self) -> Mutation {
        Mutation::RepoRemove(RepoRemoveInput {
            root: self.root().to_owned(),
        })
    }

    fn registered(&self) -> bool {
        self.service()
            .list_repositories()
            .unwrap()
            .items
            .iter()
            .any(|repository| Path::new(&repository.root) == self.root())
    }

    /// Removes the registration as another process would: through the
    /// domain operation, under an operation of its own.
    fn remove_registration_by_hand(&self) {
        self.service()
            .remove_registration(RemoveRegistrationRequest {
                root: self.root().to_owned(),
                operation_id: manyhands::repository::OperationId::new(),
            })
            .unwrap();
    }
}

fn repository_removal(preview: &PreviewDto) -> &RepositoryRemovalPreviewDto {
    match &preview.details {
        PreviewDetailsDto::RepositoryRemoval(details) => details,
        other => panic!("not a preview of a registration's removal: {other:?}"),
    }
}

#[test]
fn a_preview_of_repo_remove_names_the_registration_and_changes_nothing() {
    let home = Home::new();
    let files = tree(home.root());
    let repositories = serde_json::to_string(&home.service().list_repositories().unwrap()).unwrap();

    let (_, preview) = home.preview(&home.remove_repository());

    assert_eq!(tree(home.root()), files);
    assert_eq!(
        serde_json::to_string(&home.service().list_repositories().unwrap()).unwrap(),
        repositories
    );
    assert_eq!(home.confirmation_rows(), 1);
    assert_eq!(home.world.request_rows(), 0);
    assert_eq!(preview.command, "repo remove");
    assert_eq!(
        preview.scope.repository.as_deref(),
        Some(home.root().to_str().unwrap())
    );
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::RegistrationRemoved,
            PreviewEffect::IndexDropped,
            PreviewEffect::OperationRecordsDropped,
            PreviewEffect::RemoteStateDropped,
        ]
    );
    let details = repository_removal(&preview);
    assert_eq!(details.root, home.root().to_str().unwrap());
    // What the repository list says of the same registration.
    let listed = home.service().list_repositories().unwrap();
    assert_eq!(details.enabled_at, listed.items[0].enabled_at);
    assert!(details.enabled_at.is_some());
    assert!(!details.files_changed);
    assert!(!details.git_state_changed);
}

/// The registration is removed and what was cached with it, and no file
/// and nothing of Git's is touched. A retry of the same request is given
/// the stored result at any later time, although the registration that
/// scoped it is gone.
#[test]
fn a_confirmed_repo_remove_touches_no_file_and_is_replayed_once_the_registration_is_gone() {
    let home = Home::new();
    home.add_remote(
        "origin",
        &format!("ssh://git:{URL_SENTINEL}@example.invalid/repository.git"),
    );
    let prepared = home.prepare(&home.remove_repository());
    assert_eq!(
        golden::find_sentinel(&serde_json::to_value(&prepared).unwrap(), &[URL_SENTINEL]),
        None
    );
    let confirmation = prepared.data.unwrap().confirmation_id;
    let files = tree(home.root());
    assert!(home.count("SELECT COUNT(*) FROM discovered_items") > 0);

    let removed = home.execute(REQUEST_1, &confirmation, home.remove_repository());

    assert_done(&removed);
    assert_eq!(removed.data, None);
    assert_eq!(
        removed.scope.repository.as_deref(),
        Some(home.root().to_str().unwrap())
    );
    // Every file, the Git directory's among them, is byte for byte what
    // it was.
    assert_eq!(tree(home.root()), files);
    assert!(!home.registered());
    for table in [
        "repositories",
        "contexts",
        "discovered_items",
        "operation_records",
        "remote_polling_state",
    ] {
        assert_eq!(
            home.count(&format!("SELECT COUNT(*) FROM {table}")),
            0,
            "{table}"
        );
    }
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    mutation::assert_request_stores_exclude(home.world.data.path(), &[URL_SENTINEL]);

    home.clock.set(START + 86_400);
    assert_eq!(
        home.execute(REQUEST_1, &confirmation, home.remove_repository()),
        removed
    );
    assert_eq!(home.retry(REQUEST_1, home.remove_repository()), removed);
    assert_eq!(tree(home.root()), files);

    // The same request ID for another repository is another request.
    let plain = Plain::new();
    let changed = home.retry(
        REQUEST_1,
        Mutation::RepoRemove(RepoRemoveInput {
            root: plain.root.clone(),
        }),
    );
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(changed.effects, Effects::not_requested());
    // And the confirmation is used.
    let reused = home.execute(REQUEST_2, &confirmation, home.remove_repository());
    assert_refused(&reused, ResultCode::ConfirmationUsed, &[]);
}

/// The removal happens and its result is lost. Its operation's row went
/// with the registration, so nothing says whose work it was: the retry
/// is a no-op, and is not refused for a registration that is gone.
#[test]
fn a_repo_remove_whose_result_was_lost_is_a_no_op_and_not_external_change() {
    let mut home = Home::new();
    let (confirmation, _) = home.preview(&home.remove_repository());
    let files = tree(home.root());
    home.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let lost = home.execute(REQUEST_1, &confirmation, home.remove_repository());
    assert_eq!(lost.code, ResultCode::InternalError);
    assert!(!home.registered());
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));

    home.reopen();
    home.clock.set(START + 3600);
    let answered = home.retry(REQUEST_1, home.remove_repository());

    assert_noop(&answered);
    assert_eq!(answered.data, None);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(tree(home.root()), files);
    assert_eq!(home.retry(REQUEST_1, home.remove_repository()), answered);
}

/// The journal fix does not cover `remove_registration`: a failure
/// before its transaction leaves its row pending, and so the request's
/// record accepted. The same request then removes the registration.
#[test]
fn a_repo_remove_that_fails_before_its_transaction_stays_pending_and_its_retry_succeeds() {
    let mut home = Home::new();
    home.add_remote(
        "origin",
        &format!("ssh://git:{URL_SENTINEL}@example.invalid/repository.git"),
    );
    let (confirmation, _) = home.preview(&home.remove_repository());
    let files = tree(home.root());
    home.fail_once_at(FailurePoint::BeforeRegistrationRemovalTransaction);

    let failed = home.execute(REQUEST_1, &confirmation, home.remove_repository());

    assert_eq!(failed.code, ResultCode::InternalError);
    assert_eq!(failed.outcome, Outcome::Error);
    assert_eq!(failed.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&failed), ["request.retry"]);
    assert!(home.registered());
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(home.world.pending_journal_rows(), 1);
    assert_eq!(
        golden::find_sentinel(&serde_json::to_value(&failed).unwrap(), &[URL_SENTINEL]),
        None
    );
    mutation::assert_request_stores_exclude(home.world.data.path(), &[URL_SENTINEL]);

    home.reopen();
    home.clock.set(START + 3600);
    let removed = home.retry(REQUEST_1, home.remove_repository());

    assert_done(&removed);
    assert_eq!(failed.operation_id, removed.operation_id);
    assert!(!home.registered());
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(home.count("SELECT COUNT(*) FROM operation_records"), 0);
    assert_eq!(tree(home.root()), files);
}

#[test]
fn prepare_for_a_repository_that_is_not_registered_is_a_no_op_with_no_confirmation() {
    let desk = Desk::new();
    let plain = Plain::new();
    let remove = |root: &Path| {
        Mutation::RepoRemove(RepoRemoveInput {
            root: root.to_owned(),
        })
    };

    let noop = desk.prepare(&remove(&plain.root));
    assert_eq!(noop.code, ResultCode::AlreadyApplied);
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_eq!(noop.data, None);

    let directory = tempfile::tempdir().unwrap();
    let refused = desk.prepare(&remove(directory.path()));
    assert_eq!(refused.code, ResultCode::NotRepository);
    assert_eq!(refused.data, None);
    assert_eq!(desk.confirmation_rows(), 0);
}

/// Someone else removes the registration after the preview. A request
/// that no record holds is refused: its preview is of a state that is
/// gone, and a new one says that nothing is left to do.
#[test]
fn a_registration_removed_by_another_after_the_preview_is_external_change() {
    let home = Home::new();
    let (confirmation, _) = home.preview(&home.remove_repository());
    home.remove_registration_by_hand();

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_repository());

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(
        home.prepare(&home.remove_repository()).code,
        ResultCode::AlreadyApplied
    );
}

/// A request is accepted and stops before its first effect, and someone
/// else removes the registration. The request's end state holds: the
/// retry is a no-op.
#[test]
fn a_retry_of_a_repo_remove_another_has_done_is_a_no_op() {
    let mut home = Home::new();
    let (confirmation, _) = home.preview(&home.remove_repository());
    home.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    home.execute(REQUEST_1, &confirmation, home.remove_repository());
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    home.remove_registration_by_hand();

    let answered = home.retry(REQUEST_1, home.remove_repository());

    assert_noop(&answered);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
}

#[test]
fn repo_remove_without_a_confirmation_is_confirmation_required_and_does_nothing() {
    let home = Home::new();
    home.preview(&home.remove_repository());

    let refused = home.retry(REQUEST_1, home.remove_repository());

    assert_refused(
        &refused,
        ResultCode::ConfirmationRequired,
        &["request.prepare"],
    );
    assert!(home.registered());
    home.assert_nothing_left(REQUEST_1);
}

// ---------------------------------------------------------------------
// `key delete`
// ---------------------------------------------------------------------

/// A service with no repository, timed by a clock the test sets, and a
/// home whose key store holds one key Manyhands generated, protected by
/// a passphrase that nothing may keep.
struct Keyring {
    world: KeyWorld,
    clock: Arc<TestClock>,
    /// The generated key.
    id: SharedKeyId,
}

impl Keyring {
    fn new() -> Self {
        let clock = TestClock::at(START);
        let world = KeyWorld::with_service(|data| mutation::service_with_clock(data, &clock));
        let generated = world.execute(REQUEST_3, world.generate("Work key", Some(PASSPHRASE)));
        assert_eq!(generated.code, ResultCode::Ok, "{}", generated.message);
        let id = SharedKeyId::parse(key_data(&generated).id.as_deref().unwrap()).unwrap();
        Self { world, clock, id }
    }

    /// Another service over the same data, as a new process would open.
    fn reopen(&mut self) {
        self.world.service = mutation::service_with_clock(self.world.data.path(), &self.clock);
    }

    /// Another service over the same data that fails once at `point`.
    fn fail_once_at(&mut self, point: FailurePoint) {
        self.world.service = RepositoryService::open_at_with_failure_point_for_testing(
            self.world.data.path(),
            point,
        )
        .unwrap()
        .with_clock_for_testing(self.clock.clone());
    }

    fn service(&self) -> &RepositoryService {
        &self.world.service
    }

    fn delete(&self, id: SharedKeyId) -> Mutation {
        Mutation::KeyDelete(KeyDeleteInput {
            store: self.world.store(),
            id,
        })
    }

    fn delete_key(&self) -> Mutation {
        self.delete(self.id)
    }

    /// `prepare`, with its envelope scanned for the passphrase.
    fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
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

    fn preview(&self) -> (String, PreviewDto) {
        let envelope = self.prepare(&self.delete_key());
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    fn scanned(&self, envelope: Envelope<MutationDataDto>) -> Envelope<MutationDataDto> {
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

    fn execute(
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
    fn retry(&self, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        self.scanned(execute_with(self.service(), request_id, mutation))
    }

    fn state(&self, request_id: &str) -> Option<RequestState> {
        self.world.record(request_id).map(|record| record.state)
    }

    fn count(&self, query: &str) -> i64 {
        items::index(self.world.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    /// The key's two files, as its registration names them.
    fn files(&self) -> (PathBuf, PathBuf) {
        let directory = self.world.store_directory();
        (
            directory.join(self.id.to_string()),
            directory.join(format!("{}.pub", self.id)),
        )
    }

    fn registered(&self) -> bool {
        self.world.keys().iter().any(|(id, _)| *id == self.id)
    }

    /// Nothing is left of a request that was rejected: no record, no
    /// confirmation held, and no deletion begun.
    fn assert_nothing_left(&self, request_id: &str) {
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

fn key_deletion(preview: &PreviewDto) -> &KeyDeletionPreviewDto {
    match &preview.details {
        PreviewDetailsDto::KeyDeletion(details) => details,
        other => panic!("not a preview of a key's deletion: {other:?}"),
    }
}

#[test]
fn a_preview_of_key_delete_names_the_key_and_its_files_and_changes_nothing() {
    let keyring = Keyring::new();
    let (private, public) = keyring.files();
    let store = tree(&keyring.world.store_directory());
    let listed = keyring.service().show_key(keyring.id).unwrap();

    let (_, preview) = keyring.preview();

    assert_eq!(tree(&keyring.world.store_directory()), store);
    assert_eq!(keyring.service().show_key(keyring.id).unwrap(), listed);
    assert_eq!(
        keyring.count("SELECT COUNT(*) FROM confirmation_records"),
        1
    );
    // The generation's record and row, and nothing of a deletion.
    assert_eq!(keyring.world.request_rows(), 1);
    assert_eq!(keyring.world.key_operations(), 1);
    assert_eq!(preview.command, "key delete");
    assert_eq!(preview.scope, manyhands::results::Scope::default());
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::PrivateKeyFileDeleted,
            PreviewEffect::PublicKeyFileDeleted,
            PreviewEffect::KeyUnregistered,
        ]
    );
    let details = key_deletion(&preview);
    assert_eq!(details.id, keyring.id.to_string());
    assert_eq!(details.label, "Work key");
    // No more than the key list gives of the same key.
    assert_eq!(details.fingerprint, listed.fingerprint);
    assert!(details.fingerprint.is_some());
    assert_eq!(details.private_key_path, listed.private_key_path);
    assert_eq!(details.public_key_path, listed.public_key_path);
    assert_eq!(Path::new(&details.private_key_path), private);
    assert_eq!(
        details.public_key_path.as_deref().map(Path::new),
        Some(public.as_path())
    );
    assert!(private.exists() && public.exists());
}

#[test]
fn prepare_refuses_a_key_that_is_imported_selected_or_not_registered() {
    let keyring = Keyring::new();
    let imported = keyring.world.register("id_imported", "Imported");

    let refused = keyring.prepare(&keyring.delete(imported));
    assert_eq!(refused.code, ResultCode::KeyNotDeletable);
    assert_eq!(refused.outcome, Outcome::Error);
    assert_eq!(refused.data, None);
    assert!(refused.recovery.is_empty());

    keyring.service().select_shared_key(keyring.id).unwrap();
    let selected = keyring.prepare(&keyring.delete_key());
    assert_eq!(selected.code, ResultCode::SelectedKeyInUse);
    assert_eq!(selected.outcome, Outcome::Blocked);
    assert_eq!(selected.data, None);

    let unknown = keyring.prepare(&keyring.delete(SharedKeyId::new()));
    assert_eq!(unknown.code, ResultCode::KeyNotFound);
    assert_eq!(unknown.data, None);

    assert_eq!(
        keyring.count("SELECT COUNT(*) FROM confirmation_records"),
        0
    );
    let (private, public) = keyring.files();
    assert!(private.exists() && public.exists());
}

#[test]
fn a_confirmed_key_delete_removes_the_files_and_the_key_and_is_replayed() {
    let keyring = Keyring::new();
    let imported = keyring.world.register("id_imported", "Imported");
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    let fingerprint = keyring.service().show_key(keyring.id).unwrap().fingerprint;

    let required = keyring.retry(REQUEST_1, keyring.delete_key());
    assert_refused(
        &required,
        ResultCode::ConfirmationRequired,
        &["request.prepare"],
    );
    let other = keyring.execute(REQUEST_1, &confirmation, keyring.delete(imported));
    assert_refused(&other, ResultCode::ConfirmationMismatch, &[]);
    assert!(private.exists() && public.exists());

    let deleted = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());

    assert_done(&deleted);
    let data = key_data(&deleted);
    assert_eq!(data.id, Some(keyring.id.to_string()));
    assert_eq!(data.label.as_deref(), Some("Work key"));
    assert_eq!(data.fingerprint, fingerprint);
    assert_eq!(data.generation, None);
    assert_eq!(data.deletion, None);
    assert!(deleted.operation_id.is_some());
    assert!(!private.exists() && !public.exists());
    assert!(!keyring.registered());
    // The other key and its file are as they were.
    assert!(keyring.world.keys().iter().any(|(id, _)| *id == imported));
    assert_eq!(keyring.state(REQUEST_1), Some(RequestState::Finished));

    // At any later time, whatever the retry presents: the stored result.
    keyring.clock.set(START + 86_400);
    assert_eq!(
        keyring.execute(REQUEST_1, &confirmation, keyring.delete_key()),
        deleted
    );
    assert_eq!(keyring.retry(REQUEST_1, keyring.delete_key()), deleted);

    // The same request ID for another key is another request.
    let changed = keyring.retry(REQUEST_1, keyring.delete(imported));
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(changed.effects, Effects::not_requested());
    // And the confirmation is used.
    let reused = keyring.execute(REQUEST_2, &confirmation, keyring.delete_key());
    assert_refused(&reused, ResultCode::ConfirmationUsed, &[]);
}

/// The key is selected after the preview: its registration is no longer
/// the one that was shown.
#[test]
fn a_registration_changed_between_preview_and_confirmation_is_external_change() {
    let keyring = Keyring::new();
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    keyring.service().select_shared_key(keyring.id).unwrap();

    let refused = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    assert_eq!(refused.outcome, Outcome::Blocked);
    keyring.assert_nothing_left(REQUEST_1);
    assert!(private.exists() && public.exists());
    assert!(keyring.registered());
    // A new preview says what is in the way.
    assert_eq!(
        keyring.prepare(&keyring.delete_key()).code,
        ResultCode::SelectedKeyInUse
    );
}

/// A key file is replaced after the preview. The registration is what
/// it was, so the preview's observation holds: it is the domain's own
/// review of the files that refuses, and nothing is deleted.
#[test]
fn a_key_file_changed_after_the_preview_is_external_change_from_the_domain_s_review() {
    let keyring = Keyring::new();
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    // Written in place: the file is the store's own still, with its
    // owner and its mode, and holds something else.
    fs::write(&private, "another file\n").unwrap();

    let refused = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    keyring.assert_nothing_left(REQUEST_1);
    assert_eq!(fs::read_to_string(&private).unwrap(), "another file\n");
    assert!(public.exists());
    assert!(keyring.registered());
}

/// The deletion is interrupted once its private key file is removed:
/// before that removal is recorded, and after. An hour later, with the
/// confirmation's own record of what was previewed overwritten, the same
/// request completes under the same operation: a fresh review is
/// obtained, and nothing is compared with the preview.
#[test]
fn a_deletion_interrupted_after_the_private_file_is_removed_is_partial_and_its_retry_completes() {
    for (point, state) in [
        (
            FailurePoint::DeletionAfterPrivateUnlink,
            KeyDeletionState::Prepared,
        ),
        (
            FailurePoint::DeletionAfterPrivatePhase,
            KeyDeletionState::PrivateRemoved,
        ),
        (
            FailurePoint::DeletionAfterPublicUnlink,
            KeyDeletionState::PrivateRemoved,
        ),
    ] {
        let mut keyring = Keyring::new();
        let (private, public) = keyring.files();
        let (confirmation, _) = keyring.preview();
        keyring.fail_once_at(point);

        let partial = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());

        assert_eq!(partial.code, ResultCode::RecoveryRequired, "{point:?}");
        assert_eq!(partial.outcome, Outcome::Partial, "{point:?}");
        assert_eq!(partial.effects, Effects::not_requested(), "{point:?}");
        assert_eq!(recovery_actions(&partial), ["request.retry"], "{point:?}");
        assert_eq!(key_data(&partial).deletion, Some(state), "{point:?}");
        assert_eq!(
            key_data(&partial).id,
            Some(keyring.id.to_string()),
            "{point:?}"
        );
        assert!(!private.exists(), "{point:?}");
        assert_eq!(
            public.exists(),
            point != FailurePoint::DeletionAfterPublicUnlink,
            "{point:?}"
        );
        assert!(keyring.registered(), "{point:?}");
        assert_eq!(
            keyring.state(REQUEST_1),
            Some(RequestState::Accepted),
            "{point:?}"
        );

        keyring.reopen();
        keyring.clock.set(START + 3600);
        let overwritten = items::index(keyring.world.data.path())
            .execute(
                "UPDATE confirmation_records SET observation_digest = ?1",
                ["0".repeat(64)],
            )
            .unwrap();
        assert_eq!(overwritten, 1, "{point:?}");
        let finished = keyring.retry(REQUEST_1, keyring.delete_key());

        assert_done(&finished);
        assert_eq!(partial.operation_id, finished.operation_id, "{point:?}");
        assert_eq!(key_data(&finished).deletion, None, "{point:?}");
        assert!(!private.exists() && !public.exists(), "{point:?}");
        assert!(!keyring.registered(), "{point:?}");
        assert_eq!(
            keyring.state(REQUEST_1),
            Some(RequestState::Finished),
            "{point:?}"
        );
        assert_eq!(
            keyring.count("SELECT COUNT(*) FROM key_material_operations WHERE action = 'delete'"),
            1,
            "{point:?}"
        );
        assert_eq!(
            keyring.execute(REQUEST_1, &confirmation, keyring.delete_key()),
            finished,
            "{point:?}"
        );
    }
}

/// A deletion that fails before it removed anything has begun all the
/// same: its record stays accepted, nothing is reported as changed, and
/// the same request completes it.
#[test]
fn a_deletion_that_fails_before_it_removed_a_file_is_not_partial_and_its_retry_completes() {
    let mut keyring = Keyring::new();
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    keyring.fail_once_at(FailurePoint::DeletionAfterIntent);

    let stopped = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());

    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Blocked);
    assert_eq!(recovery_actions(&stopped), ["request.retry"]);
    assert_eq!(
        key_data(&stopped).deletion,
        Some(KeyDeletionState::Prepared)
    );
    assert!(private.exists() && public.exists());
    assert_eq!(keyring.state(REQUEST_1), Some(RequestState::Accepted));

    keyring.reopen();
    assert_done(&keyring.retry(REQUEST_1, keyring.delete_key()));
    assert!(!private.exists() && !public.exists());
}

/// The deletion completes and its result is lost. The key is no longer
/// registered, so no review of it can be had: the retry is answered by
/// the deletion called with none, as a no-op, and is not refused.
#[test]
fn a_key_delete_whose_result_was_lost_is_a_no_op_and_not_external_change() {
    let mut keyring = Keyring::new();
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    keyring.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let lost = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
    assert_eq!(lost.code, ResultCode::InternalError);
    assert!(!private.exists() && !public.exists());
    assert!(!keyring.registered());
    assert_eq!(keyring.state(REQUEST_1), Some(RequestState::Accepted));

    keyring.reopen();
    keyring.clock.set(START + 3600);
    // A file at the key's path now is not the key's, and is not touched.
    fs::write(&private, "another file\n").unwrap();
    let answered = keyring.retry(REQUEST_1, keyring.delete_key());

    assert_noop(&answered);
    assert_eq!(key_data(&answered).id, Some(keyring.id.to_string()));
    assert_eq!(keyring.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(fs::read_to_string(&private).unwrap(), "another file\n");
    assert_eq!(keyring.retry(REQUEST_1, keyring.delete_key()), answered);
}

/// A request is accepted and stops before its first effect, and the key
/// is selected. The retry observes again and is refused: the consent was
/// to what was shown.
#[test]
fn a_retry_of_a_key_delete_before_any_effect_observes_again() {
    let mut keyring = Keyring::new();
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    keyring.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    let died = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(keyring.state(REQUEST_1), Some(RequestState::Accepted));
    keyring.service().select_shared_key(keyring.id).unwrap();

    let refused = keyring.retry(REQUEST_1, keyring.delete_key());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    keyring.assert_nothing_left(REQUEST_1);
    assert!(private.exists() && public.exists());
    assert!(keyring.registered());
}

/// The same interruption with nothing changed: the retry is honored,
/// also after the ten minutes.
#[test]
fn a_retry_of_a_key_delete_before_any_effect_runs_when_nothing_has_changed() {
    let mut keyring = Keyring::new();
    let (private, public) = keyring.files();
    let (confirmation, _) = keyring.preview();
    keyring.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
    keyring.clock.set(START + 3600);

    let finished = keyring.retry(REQUEST_1, keyring.delete_key());

    assert_done(&finished);
    assert!(!private.exists() && !public.exists());
}

/// The remote is deleted and the step that says so never reaches the
/// journal: the row is pending and holds no step. No fault point stands
/// between the two, so the row is put in that state by hand. An hour
/// later, with the confirmation's own record of what was previewed
/// overwritten, the same request ends its operation and compares
/// nothing: the remote it was to remove is gone. Nothing says the
/// removal was its own, so the retry is a no-op.
#[test]
fn a_removal_whose_step_never_reached_the_journal_is_completed_without_observing_again() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::AfterRemoteMutation);
    home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    let index = items::index(home.world.data.path());
    let unrecorded = index
        .execute(
            "UPDATE operation_records SET completed_step = NULL
              WHERE state != 'completed' AND action = 'remove_remote'",
            [],
        )
        .unwrap();
    assert_eq!(unrecorded, 1);
    assert_eq!(
        index
            .execute(
                "UPDATE confirmation_records SET observation_digest = ?1",
                ["0".repeat(64)],
            )
            .unwrap(),
        1
    );
    drop(index);

    home.reopen();
    home.clock.set(START + 3600);
    let answered = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_noop(&answered);
    assert_eq!(home.remote_url("backup"), None);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(home.world.pending_journal_rows(), 0);
}

// ---------------------------------------------------------------------
// `remote remove`: what Git removes by the remote's refspecs
// ---------------------------------------------------------------------

/// Every reference of the repository, by name, with what it points at.
fn references(root: &Path) -> BTreeMap<String, String> {
    let repository = git2::Repository::open(root).unwrap();
    repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            (
                reference.name().unwrap().to_owned(),
                reference
                    .target()
                    .map(|oid| oid.to_string())
                    .or_else(|| reference.symbolic_target().map(str::to_owned))
                    .unwrap(),
            )
        })
        .collect()
}

/// A remote whose refspecs name the repository's own references as
/// their destination, and a branch and a tag for Git to take with it.
/// Returns the references as they are.
fn remote_over_local_refs(
    home: &Home,
    fetch: &str,
    push: Option<&str>,
) -> BTreeMap<String, String> {
    let repository = home.git();
    repository
        .remote_with_fetch("backup", BACKUP, fetch)
        .unwrap();
    if let Some(push) = push {
        repository.remote_add_push("backup", push).unwrap();
    }
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    repository
        .tag_lightweight("v1", head.as_object(), false)
        .unwrap();
    let before = references(home.root());
    assert!(before.contains_key("refs/heads/main"));
    assert!(before.contains_key("refs/tags/v1"));
    before
}

const USUAL_FETCH: &str = "+refs/heads/*:refs/remotes/backup/*";

/// Git removes with a remote every reference that matches the
/// destination of one of its refspecs. For a mirror those are the
/// repository's branches and tags; for a push refspec onto
/// `refs/heads/*`, its branches. Before this was observed, the preview
/// of each said `remote_removed` and its execution deleted them. No
/// preview is given of such a remote, and nothing is removed.
#[test]
fn a_remote_whose_removal_would_delete_branches_or_tags_is_refused_at_prepare() {
    for (what, fetch, push) in [
        ("a mirror", "+refs/*:refs/*", None),
        (
            "a push refspec onto local branches",
            USUAL_FETCH,
            Some("refs/heads/*:refs/heads/*"),
        ),
        (
            "a fetch refspec onto another remote's references",
            "+refs/heads/*:refs/remotes/upstream/*",
            None,
        ),
    ] {
        let home = Home::new();
        remote_over_local_refs(&home, fetch, push);
        let repository = home.git();
        let head = repository.head().unwrap().peel_to_commit().unwrap().id();
        repository
            .reference("refs/remotes/upstream/main", head, false, "test")
            .unwrap();
        let before: BTreeMap<_, _> = references(home.root());
        assert!(before.len() >= 3, "{what}");

        let refused = home.prepare(&home.remove_remote("backup"));

        assert_eq!(refused.code, ResultCode::InvalidRemote, "{what}");
        assert_eq!(refused.outcome, Outcome::Error, "{what}");
        assert_eq!(refused.data, None, "{what}");
        assert_eq!(home.confirmation_rows(), 0, "{what}");
        assert_eq!(references(home.root()), before, "{what}");
        assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP), "{what}");
    }
}

/// The refspec that makes a removal destructive is added after the
/// preview. The request is refused before it is accepted, as `prepare`
/// would refuse it, and every reference is as it was.
#[test]
fn a_destructive_refspec_added_after_the_preview_is_refused_at_execute() {
    let home = Home::new();
    let before = remote_over_local_refs(&home, USUAL_FETCH, None);
    let (confirmation, preview) = home.preview(&home.remove_remote("backup"));
    assert_eq!(effects(&preview), [PreviewEffect::RemoteRemoved]);
    home.git()
        .remote_add_push("backup", "refs/heads/*:refs/heads/*")
        .unwrap();

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_refused(&refused, ResultCode::InvalidRemote, &[]);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(references(home.root()), before);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
}

/// The same, with the request already accepted when the refspec is
/// added: the retry is refused before its domain call.
#[test]
fn a_destructive_refspec_added_under_an_accepted_request_is_refused_on_its_retry() {
    let mut home = Home::new();
    let before = remote_over_local_refs(&home, USUAL_FETCH, None);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    home.git()
        .remote_add_fetch("backup", "+refs/*:refs/*")
        .unwrap();

    let refused = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_eq!(refused.code, ResultCode::InvalidRemote);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(references(home.root()), before);
    assert_eq!(home.remote_url("backup").as_deref(), Some(BACKUP));
}

/// A refspec that changes after the preview and names only the remote's
/// own tracking references is still not what was shown.
#[test]
fn a_refspec_changed_after_the_preview_is_external_change() {
    let home = Home::new();
    let before = remote_over_local_refs(&home, USUAL_FETCH, None);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.git()
        .remote_add_fetch("backup", "+refs/tags/*:refs/remotes/backup/tags/*")
        .unwrap();

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(references(home.root()), before);
}

/// What Git does remove with a remote configured in the usual way, and
/// what it leaves: the tracking references and the upstream of a branch
/// that follows the remote go; HEAD, every branch and tag, another
/// remote and its tracking references, and the upstream of a branch
/// that follows that other remote stay.
#[test]
fn a_removal_takes_the_remote_s_own_references_and_upstreams_and_nothing_else() {
    let home = Home::new();
    remote_over_local_refs(&home, USUAL_FETCH, None);
    home.add_remote("other", "git@example.invalid:team/other.git");
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    repository.branch("side", &head, false).unwrap();
    for name in ["refs/remotes/backup/main", "refs/remotes/other/main"] {
        repository
            .reference(name, head.id(), false, "test")
            .unwrap();
    }
    let mut config = repository.config().unwrap();
    for (branch, remote) in [("main", "backup"), ("side", "other")] {
        config
            .set_str(&format!("branch.{branch}.remote"), remote)
            .unwrap();
        config
            .set_str(&format!("branch.{branch}.merge"), "refs/heads/main")
            .unwrap();
    }
    let mut expected = references(home.root());
    let head_before = fs::read(home.root().join(".git/HEAD")).unwrap();
    let (confirmation, preview) = home.preview(&home.remove_remote("backup"));
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::RemoteRemoved,
            PreviewEffect::RemoteTrackingRefsRemoved,
            PreviewEffect::BranchUpstreamsRemoved,
        ]
    );

    assert_done(&home.execute(REQUEST_1, &confirmation, home.remove_remote("backup")));

    expected.remove("refs/remotes/backup/main");
    assert_eq!(references(home.root()), expected);
    assert_eq!(
        fs::read(home.root().join(".git/HEAD")).unwrap(),
        head_before
    );
    let config = home.git().config().unwrap();
    assert!(config.get_string("branch.main.remote").is_err());
    assert!(config.get_string("branch.main.merge").is_err());
    assert_eq!(config.get_string("branch.side.remote").unwrap(), "other");
    assert_eq!(
        config.get_string("branch.side.merge").unwrap(),
        "refs/heads/main"
    );
    assert!(home.remote_url("other").is_some());
    assert_eq!(home.remote_url("backup"), None);
}

/// The request removed its remote and was interrupted, and a remote of
/// the same name is added by hand before the retry. The domain operation
/// would remove whatever remote has the name: it is not called, and the
/// new remote stays. The record is kept: the request's operation is
/// still pending.
#[test]
fn a_remote_added_again_under_an_interrupted_removal_is_not_removed_by_the_retry() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::AfterRemoteMutation);
    home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_eq!(home.remote_url("backup"), None);
    home.git()
        .remote_with_fetch(
            "backup",
            "git@example.invalid:team/new.git",
            "+refs/*:refs/*",
        )
        .unwrap();
    let before = references(home.root());

    home.reopen();
    let refused = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(refused.outcome, Outcome::Partial);
    assert_offered_its_own_abandon(&refused, "the retry");
    assert_eq!(references(home.root()), before);
    assert!(home.remote_url("backup").is_some());
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));

    // With the remote that was added taken away again, by hand and
    // without Git's deletion of what its refspecs name, the request ends.
    let mut config = home.git().config().unwrap();
    config.remove("remote.backup.url").unwrap();
    config.remove("remote.backup.fetch").unwrap();
    assert_done(&home.retry(REQUEST_1, home.remove_remote("backup")));
    assert_eq!(references(home.root()), before);
    assert_eq!(home.world.pending_journal_rows(), 0);
}

// ---------------------------------------------------------------------
// `repo remove`: a root that is registered again
// ---------------------------------------------------------------------

impl Home {
    /// Registers the root again, as another process would: through the
    /// domain operation, which finds the repository enabled. The new
    /// registration is made a minute after the old one, since the time
    /// of a registration is kept in seconds.
    fn register_again(&self) {
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

/// The removal happens, its result is lost, and the root is registered
/// again before the retry. The registration the request consented to
/// removing is gone, whoever removed it: the retry is a no-op, is not
/// refused as `external_change`, and leaves the new registration.
#[test]
fn a_repo_remove_whose_result_was_lost_is_a_no_op_once_the_root_is_registered_again() {
    let mut home = Home::new();
    let (confirmation, _) = home.preview(&home.remove_repository());
    home.fail_once_at(FailurePoint::BeforeRequestSettlement);
    home.execute(REQUEST_1, &confirmation, home.remove_repository());
    assert!(!home.registered());
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    home.reopen();
    home.register_again();
    let files = tree(home.root());

    let answered = home.retry(REQUEST_1, home.remove_repository());

    assert_noop(&answered);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert!(home.registered());
    assert_eq!(tree(home.root()), files);
    assert_eq!(home.retry(REQUEST_1, home.remove_repository()), answered);
}

/// A first call is not weakened by that: a preview of a registration
/// that has since been removed and made again is stale, and a request
/// no record holds is refused with nothing removed.
#[test]
fn a_first_repo_remove_with_a_preview_of_an_earlier_registration_is_external_change() {
    let home = Home::new();
    let (confirmation, _) = home.preview(&home.remove_repository());
    home.remove_registration_by_hand();
    home.register_again();

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_repository());

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    home.assert_nothing_left(REQUEST_1);
    assert!(home.registered());
}

// ---------------------------------------------------------------------
// `key delete`: what a retry may delete
// ---------------------------------------------------------------------

/// The retry of an interrupted deletion compares nothing with the
/// preview, so this is what bounds it: it deletes the key's own files
/// and no other. A new file at the private key's path, and a public key
/// file rewritten in place, are each refused by the fresh review, with
/// every file left byte for byte as it is, the key registered and the
/// record accepted. Put back as the deletion left it, the same request
/// completes.
#[test]
fn a_retry_of_an_interrupted_deletion_deletes_nothing_that_is_not_the_key_s_own_file() {
    for what in ["a new private file", "a public file rewritten in place"] {
        let mut keyring = Keyring::new();
        let (private, public) = keyring.files();
        let (confirmation, _) = keyring.preview();
        keyring.fail_once_at(FailurePoint::DeletionAfterPrivateUnlink);
        let partial = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
        assert_eq!(partial.outcome, Outcome::Partial, "{what}");
        assert!(!private.exists() && public.exists(), "{what}");
        if what == "a new private file" {
            fs::write(&private, "someone else's file\n").unwrap();
        } else {
            fs::write(&public, "someone else's text\n").unwrap();
        }
        let store = tree(&keyring.world.store_directory());

        keyring.reopen();
        keyring.clock.set(START + 3600);
        let refused = keyring.retry(REQUEST_1, keyring.delete_key());

        assert!(refused.code.failure_class().is_some(), "{what}");
        assert_ne!(refused.outcome, Outcome::Success, "{what}");
        assert_eq!(tree(&keyring.world.store_directory()), store, "{what}");
        assert!(keyring.registered(), "{what}");
        assert_eq!(
            keyring.state(REQUEST_1),
            Some(RequestState::Accepted),
            "{what}"
        );
        // A second retry is refused the same way and deletes nothing.
        keyring.retry(REQUEST_1, keyring.delete_key());
        assert_eq!(tree(&keyring.world.store_directory()), store, "{what}");

        // With the new file taken away again the request completes. A
        // public key file that was rewritten is not the key's own again
        // for being given its old bytes: that deletion stays refused.
        if what == "a new private file" {
            fs::remove_file(&private).unwrap();
            assert_done(&keyring.retry(REQUEST_1, keyring.delete_key()));
            assert!(!private.exists() && !public.exists(), "{what}");
            assert!(!keyring.registered(), "{what}");
        }
    }
}

/// A key whose private key file is not there: the preview does not say
/// that file will be deleted, and the deletion removes the rest. A file
/// that is at the path by the time the request runs is not what was
/// shown: the request is refused, by the domain's review of a file that
/// is not the key's own before the observation is compared, and the
/// file is left.
#[test]
fn a_preview_of_key_delete_lists_only_the_files_that_are_there() {
    let keyring = Keyring::new();
    let (private, public) = keyring.files();
    let kept = fs::read(&private).unwrap();
    fs::remove_file(&private).unwrap();

    let (confirmation, preview) = keyring.preview();

    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::PublicKeyFileDeleted,
            PreviewEffect::KeyUnregistered,
        ]
    );

    fs::write(&private, &kept).unwrap();
    let refused = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
    assert!(refused.code.failure_class().is_some(), "{:?}", refused.code);
    assert_ne!(refused.outcome, Outcome::Partial);
    keyring.assert_nothing_left(REQUEST_1);
    assert_eq!(fs::read(&private).unwrap(), kept);
    assert!(public.exists());

    fs::remove_file(&private).unwrap();
    let deleted = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
    assert_done(&deleted);
    assert!(!public.exists());
    assert!(!keyring.registered());
}

/// A record of a key command that was stored before a deletion had a
/// state holds no `deletion`. It is replayed, with none.
#[test]
fn a_stored_key_result_without_a_deletion_state_is_replayed() {
    let keyring = Keyring::new();
    let (confirmation, _) = keyring.preview();
    let deleted = keyring.execute(REQUEST_1, &confirmation, keyring.delete_key());
    assert_done(&deleted);
    let index = items::index(keyring.world.data.path());
    let stripped = index
        .execute(
            "UPDATE request_records SET result_data = json_remove(result_data, '$.deletion')
              WHERE command = 'key delete'",
            [],
        )
        .unwrap();
    assert_eq!(stripped, 1);
    let stored: String = index
        .query_row(
            "SELECT result_data FROM request_records WHERE command = 'key delete'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!stored.contains("deletion"), "{stored}");
    drop(index);

    assert_eq!(keyring.retry(REQUEST_1, keyring.delete_key()), deleted);
}

/// The remote is deleted and the call fails before the step that says
/// so reaches the journal: the row is pending and holds no step. That
/// the remote is gone under the request's own pending row is what says
/// something was done: the result is `partial`, and the record stays
/// accepted. The retry compares nothing with the preview, whose record
/// is overwritten here, and ends the operation. The domain, which has no
/// step to go by, reports no change, so the retry is a no-op.
#[test]
fn a_removal_that_fails_before_its_step_is_recorded_is_partial_and_its_retry_is_a_no_op() {
    let mut home = Home::new();
    home.add_remote("backup", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.fail_once_at(FailurePoint::AfterRemoteDeletion);

    let partial = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_eq!(partial.code, ResultCode::InternalError);
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_eq!(partial.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&partial), ["request.retry"]);
    assert_eq!(home.remote_url("backup"), None);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(
        home.count(
            "SELECT COUNT(*) FROM operation_records
              WHERE state != 'completed' AND completed_step IS NULL"
        ),
        1
    );

    home.reopen();
    home.clock.set(START + 3600);
    assert_eq!(
        items::index(home.world.data.path())
            .execute(
                "UPDATE confirmation_records SET observation_digest = ?1",
                ["0".repeat(64)],
            )
            .unwrap(),
        1
    );
    let answered = home.retry(REQUEST_1, home.remove_remote("backup"));

    assert_noop(&answered);
    assert_eq!(partial.operation_id, answered.operation_id);
    assert_eq!(home.state(REQUEST_1), Some(RequestState::Finished));
    assert_eq!(home.world.pending_journal_rows(), 0);
}

// ---------------------------------------------------------------------
// `remote remove`: what Git removes of a sibling remote
// ---------------------------------------------------------------------

/// Every entry of the repository's own configuration file, in order.
fn local_entries(root: &Path) -> Vec<(String, String)> {
    let config = git2::Config::open(&root.join(".git/config")).unwrap();
    let mut entries = config.entries(None).unwrap();
    let mut found = Vec::new();
    while let Some(entry) = entries.next() {
        let entry = entry.unwrap();
        found.push((
            entry.name().unwrap().to_owned(),
            entry.value().unwrap_or_default().to_owned(),
        ));
    }
    found
}

const SIBLING: &str = "git@example.invalid:team/sibling.git";

/// A remote and a sibling whose name begins with its name, the sibling
/// with a tracking reference. Returns the sibling's configuration and
/// every reference, as they are.
fn with_sibling(
    home: &Home,
    name: &str,
    sibling: &str,
) -> (Vec<(String, String)>, BTreeMap<String, String>) {
    if home.remote_url(name).is_none() {
        home.add_remote(name, BACKUP);
    }
    home.add_remote(sibling, SIBLING);
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference(&format!("refs/remotes/{sibling}/main"), head, false, "test")
        .unwrap();
    (sibling_entries(home, sibling), references(home.root()))
}

fn sibling_entries(home: &Home, sibling: &str) -> Vec<(String, String)> {
    local_entries(home.root())
        .into_iter()
        .filter(|(key, _)| key.starts_with(&format!("remote.{sibling}.")))
        .collect()
}

/// Git removes a remote's configuration by a pattern that is not
/// anchored, and its tracking references by a pattern whose `*` crosses
/// `/`. So removing `origin` took the whole configuration of
/// `origin.bak`, and removing `team` took the tracking references of
/// `team/fork`, each behind a preview that named neither. No preview is
/// given of such a remote, and nothing is removed.
#[test]
fn a_remote_whose_removal_would_take_part_of_a_sibling_remote_is_refused_at_prepare() {
    for (name, sibling) in [("origin", "origin.bak"), ("team", "team/fork")] {
        let home = Home::new();
        let (config, refs) = with_sibling(&home, name, sibling);
        assert_eq!(config.len(), 2, "{name}");

        let refused = home.prepare(&home.remove_remote(name));

        assert_eq!(refused.code, ResultCode::InvalidRemote, "{name}");
        assert_eq!(refused.data, None, "{name}");
        assert_eq!(home.confirmation_rows(), 0, "{name}");
        assert_eq!(sibling_entries(&home, sibling), config, "{name}");
        assert_eq!(references(home.root()), refs, "{name}");
        assert_eq!(home.remote_url(name).as_deref(), Some(BACKUP), "{name}");
    }
}

/// `origin.bak` beside `origin` takes nothing of `origin`: neither its
/// configuration nor a reference its refspec names. It is removed, and
/// `origin` is whole.
#[test]
fn a_sibling_whose_removal_takes_nothing_of_the_other_remote_is_removed() {
    let home = Home::new();
    with_sibling(&home, "origin", "origin.bak");
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference("refs/remotes/origin/main", head, false, "test")
        .unwrap();
    let of_origin = |home: &Home| -> Vec<(String, String)> {
        local_entries(home.root())
            .into_iter()
            .filter(|(key, _)| key == "remote.origin.url" || key == "remote.origin.fetch")
            .collect()
    };
    let before = of_origin(&home);
    assert_eq!(before.len(), 2);
    let mut refs = references(home.root());

    let (confirmation, _) = home.preview(&home.remove_remote("origin.bak"));
    assert_done(&home.execute(REQUEST_1, &confirmation, home.remove_remote("origin.bak")));

    refs.remove("refs/remotes/origin.bak/main");
    assert_eq!(references(home.root()), refs);
    assert_eq!(of_origin(&home), before);
    assert_eq!(home.remote_url("origin.bak"), None);
}

/// A reference that another remote's fetch refspec names as well cannot
/// be called this remote's own. `refs/remotes/team/fork/main` is the
/// tracking reference of `team/fork`'s branch `main` and of `team`'s
/// branch `fork/main`: removing `team/fork` would delete it, and is
/// refused. So is the removal of a remote whose tracking references a
/// remote of an unrelated name also fetches into.
#[test]
fn a_reference_another_remote_also_fetches_into_is_not_removed_with_this_one() {
    for (name, other, other_fetch) in [
        ("team/fork", "team", "+refs/heads/*:refs/remotes/team/*"),
        ("backup", "elsewhere", "+refs/heads/*:refs/remotes/backup/*"),
    ] {
        let home = Home::new();
        let repository = home.git();
        repository
            .remote_with_fetch(other, SIBLING, other_fetch)
            .unwrap();
        home.add_remote(name, BACKUP);
        let head = repository.head().unwrap().peel_to_commit().unwrap().id();
        repository
            .reference(&format!("refs/remotes/{name}/main"), head, false, "test")
            .unwrap();
        let refs = references(home.root());
        let entries = local_entries(home.root());

        let refused = home.prepare(&home.remove_remote(name));

        assert_eq!(refused.code, ResultCode::InvalidRemote, "{name}");
        assert_eq!(home.confirmation_rows(), 0, "{name}");
        assert_eq!(references(home.root()), refs, "{name}");
        assert_eq!(local_entries(home.root()), entries, "{name}");
    }
}

/// `team/fork` beside a `team` whose refspec does not reach its
/// references, and with no `team` at all, is removed, and everything
/// else is whole.
#[test]
fn a_remote_under_another_s_name_whose_references_are_its_alone_is_removed() {
    for team in [Some("+refs/heads/main:refs/remotes/team/main"), None] {
        let home = Home::new();
        let repository = home.git();
        if let Some(fetch) = team {
            repository
                .remote_with_fetch("team", SIBLING, fetch)
                .unwrap();
        }
        home.add_remote("team/fork", BACKUP);
        let head = repository.head().unwrap().peel_to_commit().unwrap().id();
        for name in ["refs/remotes/team/fork/main", "refs/remotes/team/main"] {
            repository.reference(name, head, false, "test").unwrap();
        }
        let of_team = sibling_entries(&home, "team");
        let mut refs = references(home.root());

        let (confirmation, _) = home.preview(&home.remove_remote("team/fork"));
        assert_done(&home.execute(REQUEST_1, &confirmation, home.remove_remote("team/fork")));

        refs.remove("refs/remotes/team/fork/main");
        assert_eq!(references(home.root()), refs, "{team:?}");
        assert_eq!(sibling_entries(&home, "team"), of_team, "{team:?}");
    }
}

/// The refspec that makes another remote fetch into this one's
/// references is added after the preview: the request is refused before
/// it is accepted.
#[test]
fn a_refspec_added_to_another_remote_after_the_preview_is_refused_at_execute() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    home.add_remote("elsewhere", SIBLING);
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference("refs/remotes/backup/main", head, false, "test")
        .unwrap();
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    repository
        .remote_add_fetch("elsewhere", "+refs/heads/*:refs/remotes/backup/*")
        .unwrap();
    let refs = references(home.root());

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));

    assert_refused(&refused, ResultCode::InvalidRemote, &[]);
    home.assert_nothing_left(REQUEST_1);
    assert_eq!(references(home.root()), refs);

    // One that reaches none of its references is still not what was
    // shown.
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    home.add_remote("elsewhere", SIBLING);
    let (confirmation, _) = home.preview(&home.remove_remote("backup"));
    home.git()
        .remote_add_fetch("elsewhere", "+refs/tags/*:refs/remotes/elsewhere/tags/*")
        .unwrap();
    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("backup"));
    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
}

/// The sibling is added after the preview: the request is refused
/// before it is accepted. Added under a request that is accepted and
/// has done nothing: its retry is refused before the domain call.
#[test]
fn a_sibling_remote_added_after_the_preview_is_refused_at_execute_and_on_a_retry() {
    for (name, sibling) in [("origin", "origin.bak"), ("team", "team/fork")] {
        let home = Home::new();
        home.add_remote(name, BACKUP);
        let (confirmation, _) = home.preview(&home.remove_remote(name));
        let (config, refs) = with_sibling(&home, name, sibling);

        let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote(name));

        assert_refused(&refused, ResultCode::InvalidRemote, &[]);
        home.assert_nothing_left(REQUEST_1);
        assert_eq!(sibling_entries(&home, sibling), config, "{name}");
        assert_eq!(references(home.root()), refs, "{name}");

        let mut home = Home::new();
        home.add_remote(name, BACKUP);
        let (confirmation, _) = home.preview(&home.remove_remote(name));
        home.fail_once_at(FailurePoint::BeforeRequestDomainCall);
        home.execute(REQUEST_1, &confirmation, home.remove_remote(name));
        assert_eq!(home.state(REQUEST_1), Some(RequestState::Accepted));
        let (config, refs) = with_sibling(&home, name, sibling);

        let refused = home.retry(REQUEST_1, home.remove_remote(name));

        assert_eq!(refused.code, ResultCode::InvalidRemote, "{name}");
        home.assert_nothing_left(REQUEST_1);
        assert_eq!(sibling_entries(&home, sibling), config, "{name}");
        assert_eq!(references(home.root()), refs, "{name}");
        assert_eq!(home.remote_url(name).as_deref(), Some(BACKUP), "{name}");
    }
}

/// A sibling that has no tracking reference yet has nothing under the
/// remote's prefix for Git to take, and its name is still observed: one
/// that is added after the preview is not what was shown.
#[test]
fn a_sibling_remote_with_nothing_to_lose_is_still_part_of_what_was_observed() {
    let home = Home::new();
    home.add_remote("team", BACKUP);
    let (confirmation, _) = home.preview(&home.remove_remote("team"));
    home.add_remote("team/fork", SIBLING);
    let config = sibling_entries(&home, "team/fork");

    let refused = home.execute(REQUEST_1, &confirmation, home.remove_remote("team"));

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    // A new preview is given, and the removal leaves the sibling whole.
    let (confirmation, _) = home.preview(&home.remove_remote("team"));
    assert_done(&home.execute(REQUEST_1, &confirmation, home.remove_remote("team")));
    assert_eq!(sibling_entries(&home, "team/fork"), config);
    assert_eq!(home.remote_url("team/fork").as_deref(), Some(SIBLING));
}

/// The same unanchored pattern takes any entry whose name holds the
/// remote's section: here the upstream of a branch named
/// `remote.backup`. That is refused too.
#[test]
fn a_remote_whose_section_name_occurs_in_another_entry_is_refused() {
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    let mut config = home.git().config().unwrap();
    config
        .set_str("branch.remote.backup.merge", "refs/heads/main")
        .unwrap();
    let entries = local_entries(home.root());

    let refused = home.prepare(&home.remove_remote("backup"));

    assert_eq!(refused.code, ResultCode::InvalidRemote);
    assert_eq!(home.confirmation_rows(), 0);
    assert_eq!(local_entries(home.root()), entries);
}

/// A push refspec with no destination has its source as its
/// destination: `push = refs/heads/main` names that branch, and
/// `push = refs/heads/*` every branch. Both are refused.
#[test]
fn a_push_refspec_with_no_destination_is_refused_for_the_branches_it_names() {
    for push in ["refs/heads/main", "refs/heads/*"] {
        let home = Home::new();
        let before = remote_over_local_refs(&home, USUAL_FETCH, Some(push));

        let refused = home.prepare(&home.remove_remote("backup"));

        assert_eq!(refused.code, ResultCode::InvalidRemote, "{push}");
        assert_eq!(home.confirmation_rows(), 0, "{push}");
        assert_eq!(references(home.root()), before, "{push}");
    }
}

/// A negative refspec has no destination and names nothing Git removes:
/// it is no reason to refuse, and the removal leaves every branch.
#[test]
fn a_negative_refspec_does_not_refuse_a_removal_by_itself() {
    let home = Home::new();
    let mut expected = remote_over_local_refs(&home, USUAL_FETCH, None);
    let repository = home.git();
    repository
        .remote_add_fetch("backup", "^refs/heads/main")
        .unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference("refs/remotes/backup/topic", head, false, "test")
        .unwrap();

    let (confirmation, preview) = home.preview(&home.remove_remote("backup"));
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::RemoteRemoved,
            PreviewEffect::RemoteTrackingRefsRemoved,
        ]
    );
    assert_done(&home.execute(REQUEST_1, &confirmation, home.remove_remote("backup")));

    expected.remove("refs/remotes/backup/topic");
    assert_eq!(references(home.root()), expected);
    assert_eq!(home.remote_url("backup"), None);
}

/// A setting of the remote that is in the user's global configuration
/// and not in the repository's own. Git removes a remote's settings from
/// the repository's configuration only, and fails on one that is not
/// there, after it has removed the remote's references and upstream
/// settings: a removal half done. Such a remote is refused, and nothing
/// of it is touched.
#[test]
fn a_remote_with_a_setting_outside_the_repository_s_configuration_is_refused() {
    let Some(git_home) = in_a_process_with_its_own_git_home(
        "a_remote_with_a_setting_outside_the_repository_s_configuration_is_refused",
    ) else {
        return;
    };
    fs::write(
        git_home.join(".gitconfig"),
        "[remote \"backup\"]\n\tprune = true\n",
    )
    .unwrap();
    let home = Home::new();
    home.add_remote("backup", BACKUP);
    let repository = home.git();
    let head = repository.head().unwrap().peel_to_commit().unwrap().id();
    repository
        .reference("refs/remotes/backup/main", head, false, "test")
        .unwrap();
    let refs = references(home.root());
    let entries = local_entries(home.root());
    let global = fs::read(git_home.join(".gitconfig")).unwrap();

    let refused = home.prepare(&home.remove_remote("backup"));

    assert_eq!(refused.code, ResultCode::InvalidRemote);
    assert_eq!(refused.data, None);
    assert_eq!(home.confirmation_rows(), 0);
    assert_eq!(references(home.root()), refs);
    assert_eq!(local_entries(home.root()), entries);
    assert_eq!(fs::read(git_home.join(".gitconfig")).unwrap(), global);
}

// ---------------------------------------------------------------------
// `operation abandon`
// ---------------------------------------------------------------------

/// The way out for a pending local operation that its own repeat cannot
/// clear.
#[path = "mutation_confirmation/abandon.rs"]
mod abandon;
