//! Two-phase confirmation: `prepare` gives a preview and a confirmation,
//! and `execute` runs a command that needs one only for a request that
//! presents it. Driven through `repo create` and `repo enable`.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use manyhands::{
    canonical,
    repository::{
        CONFIRMATION_LIFETIME_SECONDS, CommitIdentity, FailurePoint, IndexRefreshInput, Mutation,
        MutationDataDto, PreviewDetailsDto, PreviewDto, PreviewEffect, PreviewIdentitySource,
        RepoCreateInput, RepoEnableInput, RepositoryPreviewDto, RepositoryService, RequestId,
        RequestState, request_store::RequestRecord,
    },
    results::{CheckpointEffect, DiscoveryEffect, Envelope, Outcome, ResultCode, WriteEffect},
};

mod support;

use support::{
    TestRepository, born_repository, golden,
    items::{self, TICKET_A},
    mutation::{
        self, REQUEST_1, REQUEST_2, REQUEST_3, TestClock, World, execute_confirmed, execute_with,
        recovery_actions,
    },
    schema,
};

const PREVIEW_SCHEMA: &str = "preview.schema.json";

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
        let envelope = self.service.prepare(mutation);
        let value = serde_json::to_value(&envelope).unwrap();
        schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
        golden::assert_registered_recovery("prepare", &value);
        assert_eq!(envelope.command, mutation.command());
        // A preview is of no request.
        assert_eq!(envelope.request_id, None);
        assert_eq!(envelope.operation_id, None);
        if !value["data"].is_null() {
            schema::check_published(PREVIEW_SCHEMA, &value["data"]).unwrap();
        }
        envelope
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
    let PreviewDetailsDto::Repository(details) = &preview.details;
    details
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

/// A preview holds nothing: with previews outstanding, a save and an
/// `index refresh` on a repository of the same service succeed, and the
/// previews are as good afterwards.
#[test]
fn an_outstanding_preview_holds_no_lock() {
    let world = World::new();
    let plain = Plain::new();
    let parent = tempfile::tempdir().unwrap();
    let create = || {
        Mutation::RepoCreate(RepoCreateInput {
            root: parent.path().join("new"),
            primary_branch: "main".to_owned(),
            identity: Some(identity()),
            slug_prefix: None,
        })
    };
    let previewed = |mutation: &Mutation| {
        world
            .service
            .prepare(mutation)
            .data
            .expect("a preview")
            .confirmation_id
    };
    let enable_confirmation = previewed(&plain.enable());
    let create_confirmation = previewed(&create());

    let token = world.token(TICKET_A);
    let saved = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(saved.outcome, Outcome::Success, "{}", saved.message);
    let refreshed = world.execute(
        REQUEST_2,
        Mutation::IndexRefresh(IndexRefreshInput {
            root: world.root.clone(),
        }),
    );
    assert_eq!(refreshed.outcome, Outcome::Success, "{}", refreshed.message);

    assert_enabled(
        &execute_confirmed(
            &world.service,
            REQUEST_3,
            &enable_confirmation,
            plain.enable(),
        ),
        &plain.root,
    );
    assert_enabled(
        &execute_confirmed(
            &world.service,
            "01ARZ3NDEKTSV4RRFFQ69G5FX4",
            &create_confirmation,
            create(),
        ),
        &parent.path().canonicalize().unwrap().join("new"),
    );
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
