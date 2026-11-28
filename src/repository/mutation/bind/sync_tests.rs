use std::path::Path;

use super::{
    super::super::{
        super::{
            REGISTRY_FILE, RemoteRefPlan, RemoteReservationOutcome, SynchronizationError,
            SynchronizationTarget, remote::state,
        },
        identity::ScopeKey,
        progress::{NoSession, Subject},
    },
    *,
};

/// What `stopped` is called with for a synchronization whose operation
/// row ended as cancelled, as `ended_if_unresolved` leaves it.
struct Ended {
    _data: tempfile::TempDir,
    _root: tempfile::TempDir,
    service: RepositoryService,
    scope: ScopeKey,
    accepted: Accepted,
    binding: SyncBinding,
}

impl Ended {
    fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let scope = ScopeKey::for_repository(root.path()).unwrap();
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .execute(
                "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
                [scope.as_str()],
            )
            .unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        state::with_transaction(&service, Path::new(scope.as_str()), |tx, id| {
            state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
        })
        .unwrap();
        let target = SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        };
        let operation_id = OperationId::new();
        let owner = match service
            .reserve_remote_operation(
                Path::new(scope.as_str()),
                operation_id,
                &target.operation_target(&plan),
            )
            .unwrap()
        {
            RemoteReservationOutcome::Reserved(owner) => owner,
            other => panic!("{other:?}"),
        };
        service
            .request_synchronization_cancellation(Path::new(scope.as_str()), &owner)
            .unwrap();
        service
            .check_synchronization_requests(Path::new(scope.as_str()), &owner)
            .unwrap();
        let accepted = Accepted {
            request_id: RequestId::new(),
            attempt: 1,
            scope: scope.clone(),
            family: Some(OperationFamily::Remote),
            operation_id,
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
        };
        let binding = SyncBinding {
            root: root.path().to_owned(),
            target,
            republish: None,
        };
        Self {
            _data: data,
            _root: root,
            service,
            scope,
            accepted,
            binding,
        }
    }

    /// What the binding answers `error` over the ended row.
    fn stopped(&self, error: &SynchronizationError) -> Ran {
        let call = CallContext {
            service: &self.service,
            session: &NoSession,
            confirmation: None,
        };
        let watch = Watch::new(
            &self.accepted,
            &call,
            Subject::of(&self.binding, &self.accepted),
            None,
            None,
        );
        self.binding
            .stopped(&self.service, &self.accepted, &watch, error)
    }
}

/// A finished (cancelled) context sync, as `ended_if_unresolved` leaves it,
/// is answered by the binding with the repair, and settles as Finish.
#[test]
fn unresolved_history_is_answered_with_the_repair_and_no_retry() {
    let ended = Ended::new();
    let scope = &ended.scope;
    let ran = ended.stopped(&SynchronizationError::HistoryUnresolved);
    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    let actions: Vec<_> = ran
        .answer
        .recovery
        .iter()
        .map(|action| (action.action, action.arguments.clone()))
        .collect();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].0, RecoveryActionKind::RemoteHistory);
    assert_eq!(
        actions[0].1.get("root").and_then(|root| root.as_str()),
        Some(scope.as_str())
    );
    assert!(matches!(
        ran.standing,
        Standing::Stopped {
            journal: Some(JournalRow::Final {
                kind: FinalKind::Cancelled,
                ..
            })
        }
    ));
    // The same row under any other error is the caller's own cancellation.
    let ran = ended.stopped(&SynchronizationError::RecoveryRequired);
    assert_eq!(ran.answer.code, ResultCode::Cancelled);
}

/// A ticket context's refused push (R-30) ends its operation as a cancelled
/// one too, and is answered as itself: `ticket_closed`, not the caller's
/// cancellation.
#[test]
fn a_ticket_closed_refusal_is_answered_as_itself() {
    let ended = Ended::new();
    let ran = ended.stopped(&SynchronizationError::TicketClosed);
    assert_eq!(ran.answer.code, ResultCode::TicketClosed);
    assert_ne!(ran.answer.code, ResultCode::Cancelled);
    assert!(matches!(
        ran.standing,
        Standing::Stopped {
            journal: Some(JournalRow::Final {
                kind: FinalKind::Cancelled,
                ..
            })
        }
    ));
}

/// A pull-only refusal ends its operation as a cancelled one (design Part
/// D), and is answered as itself: its own code, blocked, with no recovery
/// action, every effect `not_requested` (`effects_so_far(Unknown)` would
/// say the publication is pending), and its data. It is not the caller's
/// cancellation.
#[test]
fn local_commits_refusal_is_answered_as_itself() {
    let ended = Ended::new();
    let first = git2::Oid::from_str("0123456789abcdef0123456789abcdef01234567").unwrap();
    let second = git2::Oid::from_str("fedcba9876543210fedcba9876543210fedcba98").unwrap();
    for (commits, complete) in [
        (
            vec![
                (first, "Add a thing".to_owned()),
                (second, "Fix\tit".to_owned()),
            ],
            false,
        ),
        // A known limit: the push endpoint ahead of the fetched primary
        // refuses with nothing to list.
        (Vec::new(), true),
    ] {
        let ran = ended.stopped(&SynchronizationError::PrimaryHasLocalCommits {
            commits: commits.clone(),
            complete,
        });
        let answer = &ran.answer;
        assert_eq!(answer.code, ResultCode::PrimaryHasLocalCommits);
        assert_ne!(answer.code, ResultCode::Cancelled);
        assert!(answer.recovery.is_empty());
        assert_eq!(answer.effects, Effects::not_requested());
        assert!(!answer.local_change);
        let Some(MutationDataDto::PrimaryLocalCommits(data)) = &answer.data else {
            panic!("{:?}", answer.data)
        };
        assert_eq!(
            data.r#type,
            crate::repository::mutation::dto::PrimaryLocalCommitsType::PrimaryLocalCommits
        );
        assert_eq!(data.complete, complete);
        assert_eq!(
            data.local_commits
                .iter()
                .map(|commit| (commit.oid.clone(), commit.summary.clone()))
                .collect::<Vec<_>>(),
            commits
                .iter()
                .map(|(oid, summary)| (oid.to_string(), summary.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            data.local_commits
                .iter()
                .all(|commit| commit.oid.len() == 40),
            "{data:?}"
        );
        // The envelope the answer makes is blocked.
        assert_eq!(
            answer.code.failure_class(),
            Some(crate::results::FailureClass::Blocked)
        );
        assert!(matches!(
            ran.standing,
            Standing::Stopped {
                journal: Some(JournalRow::Final {
                    kind: FinalKind::Cancelled,
                    ..
                })
            }
        ));
    }
}

/// A synchronization refused because another operation's push is one
/// nothing answered, as `push_absence` names it (the error its boundary
/// gives, `remote::sync`), is answered `recovery_required` naming that
/// operation: the envelope carries it as `data.blocker` and offers
/// `operation.show`, and its abandon where it closes it
/// (`outcome_tests`). Here the operation is a synchronization a
/// cancellation ended as its push returned, which nothing else names.
#[test]
fn a_synchronization_refused_for_another_s_unanswered_push_names_it() {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let scope = ScopeKey::for_repository(root.path()).unwrap();
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute(
        "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
        [scope.as_str()],
    )
    .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, Path::new(scope.as_str()), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };
    let reserve = |operation_id| match service
        .reserve_remote_operation(
            Path::new(scope.as_str()),
            operation_id,
            &target.operation_target(&plan),
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("{other:?}"),
    };
    let stranded = OperationId::new();
    drop(reserve(stranded));
    db.execute(
        "UPDATE remote_operation_records
            SET phase = 'cancelled', outcome = 'cancelled', sync_checkpoint = 'push_returned',
                completed_step = 'before_push', expected_oid = ?2, local_oid = ?2,
                primary_tracking_oid = ?2, push_oid = ?2
          WHERE operation_ulid = ?1",
        [
            stranded.to_string(),
            "1111111111111111111111111111111111111111".to_owned(),
        ],
    )
    .unwrap();
    let operation_id = OperationId::new();
    drop(reserve(operation_id));
    let accepted = Accepted {
        request_id: RequestId::new(),
        attempt: 1,
        scope: scope.clone(),
        family: Some(OperationFamily::Remote),
        operation_id,
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    };
    let call = CallContext {
        service: &service,
        session: &NoSession,
        confirmation: None,
    };
    let binding = SyncBinding {
        root: root.path().to_owned(),
        target,
        republish: None,
    };
    let watch = Watch::new(
        &accepted,
        &call,
        Subject::of(&binding, &accepted),
        None,
        None,
    );
    let refused = SynchronizationError::Repository(state::recovery_required().with_blocker(Some(
        crate::repository::Blocker {
            family: OperationFamily::Remote,
            operation_id: stranded,
            live: None,
            seen: None,
        },
    )));

    let ran = binding.stopped(&service, &accepted, &watch, &refused);

    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    let blocker = ran.answer.blocker.expect("the operation in its way");
    assert_eq!(blocker.operation_id, stranded.to_string());
    assert_eq!(blocker.family, OperationFamily::Remote);
    // It holds no reservation: no call works under it.
    assert_eq!(blocker.live, Some(false));
}

const CLOSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const PULL_REQUEST: &str = "format_version = 2\nintegration = \"pull_request\"\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n\n[forge]\npull_request_url = \"https://forge.example/new?from={source}&to={target}\"\n";
const DIRECT: &str =
    "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
const OPEN_TICKET: &str = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: A ticket\n---\nBody\n";
const CLOSED_TICKET: &str = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: A ticket\nclosed_at: '2026-11-25T10:00:00Z'\nclosed_by: A <a@example.invalid>\n---\nBody\n";

/// A commit of exactly `files`, on `parent`, made with no working tree.
fn commit_files(
    repository: &git2::Repository,
    parent: Option<git2::Oid>,
    files: &[(String, &str)],
) -> git2::Oid {
    // The repository's index, never written: only its tree is.
    let mut index = repository.index().unwrap();
    index.clear().unwrap();
    for (path, content) in files {
        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: content.len() as u32,
            id: git2::Oid::zero(),
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        };
        index.add_frombuffer(&entry, content.as_bytes()).unwrap();
    }
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
    let parents: Vec<git2::Commit<'_>> = parent
        .map(|parent| repository.find_commit(parent).unwrap())
        .into_iter()
        .collect();
    repository
        .commit(
            None,
            &signature,
            &signature,
            "commit",
            &tree,
            &parents.iter().collect::<Vec<_>>(),
        )
        .unwrap()
}

/// The request an `item sync` of the repository at `root` was accepted as.
fn accepted_in(root: &Path) -> Accepted {
    Accepted {
        request_id: RequestId::new(),
        attempt: 1,
        scope: ScopeKey::for_repository(root).unwrap(),
        family: Some(OperationFamily::Remote),
        operation_id: OperationId::new(),
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    }
}

/// A registered repository holding `configuration`, whose primary and the
/// remote's tracking ref hold `primary` of the ticket, and whose ticket's
/// editing branch holds `context` of it.
struct Closing {
    _data: tempfile::TempDir,
    root: tempfile::TempDir,
    service: RepositoryService,
    /// The tip of the ticket's editing branch.
    context: git2::Oid,
}

impl Closing {
    fn new(configuration: &str, primary: &str, context: &str) -> Self {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = git2::Repository::init_opts(root.path(), &options).unwrap();
        repository
            .remote("origin", "ssh://git@example.invalid/team/repo.git")
            .unwrap();
        std::fs::create_dir_all(root.path().join(".manyhands")).unwrap();
        std::fs::write(
            root.path().join(crate::canonical::CONFIG_PATH),
            configuration,
        )
        .unwrap();
        let ticket = format!(".manyhands/tickets/{CLOSING}/ticket.md");
        let config = (crate::canonical::CONFIG_PATH.to_owned(), configuration);
        let main = commit_files(
            &repository,
            None,
            &[config.clone(), (ticket.clone(), primary)],
        );
        let context = commit_files(&repository, Some(main), &[config, (ticket, context)]);
        for (name, oid) in [
            ("refs/heads/main", main),
            ("refs/remotes/origin/main", main),
            (&format!("refs/heads/manyhands/ticket/{CLOSING}"), context),
        ] {
            repository.reference(name, oid, true, "fixture").unwrap();
        }
        let service = RepositoryService::open_at(data.path()).unwrap();
        let scope = ScopeKey::for_repository(root.path()).unwrap();
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .execute(
                "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
                [scope.as_str()],
            )
            .unwrap();
        Self {
            _data: data,
            root,
            service,
            context,
        }
    }

    fn binding(&self, target: SynchronizationTarget) -> SyncBinding {
        SyncBinding {
            root: self.root.path().to_owned(),
            target,
            republish: None,
        }
    }

    fn ticket() -> SynchronizationTarget {
        SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: CLOSING.parse().unwrap(),
        }
    }

    /// What `item sync` answers for `result`, and the link row afterwards.
    fn finished(
        &self,
        binding: &SyncBinding,
        result: &SynchronizationResult,
    ) -> (Ran, Option<crate::forge::PullRequestLink>) {
        let accepted = accepted_in(self.root.path());
        let ran = binding.finished(&self.service, &accepted, result);
        let link = self
            .service
            .read_session(RepositoryOperation::Read, |db| {
                let id: i64 = db.query_row("SELECT id FROM repositories", [], |row| row.get(0))?;
                Ok(crate::repository::pull_request_links::link_of(
                    db, id, "ticket", CLOSING,
                )?)
            })
            .unwrap();
        (ran, link)
    }

    fn published(&self) -> SynchronizationResult {
        SynchronizationResult::Complete(SynchronizationOutcome::Published {
            target: Self::ticket(),
            oid: self.context,
        })
    }

    fn current(&self) -> SynchronizationResult {
        SynchronizationResult::Complete(SynchronizationOutcome::AlreadyCurrent {
            target: Self::ticket(),
            oid: self.context,
        })
    }
}

/// `item sync` of a ticket in a pull-request repository that publishes, or
/// finds current, a branch whose ticket holds a closure the primary does not
/// answers with the pull request link and gives the link record its row
/// (design Part H, decision 12). Every other answer is unchanged: no data and
/// no row.
#[test]
fn item_sync_gives_the_link_of_a_closing_ticket() {
    let closing = Closing::new(PULL_REQUEST, OPEN_TICKET, CLOSED_TICKET);
    let (ran, link) = closing.finished(&closing.binding(Closing::ticket()), &closing.published());
    assert_eq!(ran.answer.code, ResultCode::Ok);
    let Some(MutationDataDto::PullRequestLink(data)) = &ran.answer.data else {
        panic!("{:?}", ran.answer.data)
    };
    let url = data.pull_request.url.clone().expect("the forge's template");
    assert!(
        url.starts_with("https://forge.example/new?from=") && url.ends_with("&to=main"),
        "{url}"
    );
    assert_eq!(
        data.pull_request.source,
        format!("manyhands/ticket/{CLOSING}")
    );
    assert_eq!(data.pull_request.target, "main");
    // The row is the link the answer shows.
    let link = link.expect("a row");
    assert_eq!(
        crate::repository::PullRequestDto::from(&link),
        data.pull_request
    );
    // What the publication did is told as it is of any `item sync`.
    assert_eq!(ran.answer.effects.publication, PublicationEffect::Published);
    assert_eq!(
        ran.answer.effects.commit_oid,
        Some(closing.context.to_string())
    );

    // A branch the remote already holds gives the link again.
    let again = Closing::new(PULL_REQUEST, OPEN_TICKET, CLOSED_TICKET);
    let (ran, link) = again.finished(&again.binding(Closing::ticket()), &again.current());
    assert!(matches!(
        ran.answer.data,
        Some(MutationDataDto::PullRequestLink(_))
    ));
    assert!(link.is_some());

    // Anything else is as it was: a ticket that holds no closure, one the
    // primary holds the closure of (it is closed, not closing), a direct
    // repository, another item's or the primary's synchronization, and a
    // republish.
    let anyone = |closing: &Closing, binding: SyncBinding, result| {
        let (ran, link) = closing.finished(&binding, &result);
        assert!(ran.answer.data.is_none(), "{:?}", ran.answer.data);
        assert!(link.is_none());
    };
    let open = Closing::new(PULL_REQUEST, OPEN_TICKET, OPEN_TICKET);
    anyone(&open, open.binding(Closing::ticket()), open.published());
    let closed = Closing::new(PULL_REQUEST, CLOSED_TICKET, CLOSED_TICKET);
    anyone(
        &closed,
        closed.binding(Closing::ticket()),
        closed.published(),
    );
    let direct = Closing::new(DIRECT, OPEN_TICKET, CLOSED_TICKET);
    anyone(
        &direct,
        direct.binding(Closing::ticket()),
        direct.published(),
    );
    let primary = Closing::new(PULL_REQUEST, OPEN_TICKET, CLOSED_TICKET);
    anyone(
        &primary,
        primary.binding(SynchronizationTarget::Primary),
        primary.published(),
    );
    let document = Closing::new(PULL_REQUEST, OPEN_TICKET, CLOSED_TICKET);
    anyone(
        &document,
        document.binding(SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: CLOSING.parse().unwrap(),
        }),
        document.published(),
    );
    let pending = Closing::new(PULL_REQUEST, OPEN_TICKET, CLOSED_TICKET);
    anyone(
        &pending,
        pending.binding(Closing::ticket()),
        SynchronizationResult::Complete(SynchronizationOutcome::PublishPending {
            target: Closing::ticket(),
            local_oid: pending.context,
            reason: crate::repository::PublishPendingReason::NoPublicationRemote,
        }),
    );
}

const NOTE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const NOTE_PATH: &str = "docs/note.md";
const NOTE_SOURCE: &str = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAW\ntitle: A note\n---\n\nBody\n";

/// A registered repository holding `configuration`, whose document `NOTE` is
/// on its editing branch and, when `held`, on the primary and the remote's
/// tracking ref too. `link` is the given time of the document's link row, when
/// it has one.
struct Documented {
    _data: tempfile::TempDir,
    root: tempfile::TempDir,
    service: RepositoryService,
    /// The tip of the document's editing branch.
    context: git2::Oid,
}

impl Documented {
    fn new(configuration: &str, held: bool, link: Option<i64>) -> Self {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = git2::Repository::init_opts(root.path(), &options).unwrap();
        repository
            .remote("origin", "ssh://git@example.invalid/team/repo.git")
            .unwrap();
        std::fs::create_dir_all(root.path().join(".manyhands")).unwrap();
        std::fs::write(
            root.path().join(crate::canonical::CONFIG_PATH),
            configuration,
        )
        .unwrap();
        let config = (crate::canonical::CONFIG_PATH.to_owned(), configuration);
        let note = (NOTE_PATH.to_owned(), NOTE_SOURCE);
        let main = commit_files(
            &repository,
            None,
            &if held {
                vec![config.clone(), note.clone()]
            } else {
                vec![config.clone()]
            },
        );
        let context = commit_files(&repository, Some(main), &[config, note]);
        for (name, oid) in [
            ("refs/heads/main", main),
            ("refs/remotes/origin/main", main),
            (&format!("refs/heads/manyhands/document/{NOTE}"), context),
        ] {
            repository.reference(name, oid, true, "fixture").unwrap();
        }
        let service = RepositoryService::open_at(data.path()).unwrap();
        let scope = ScopeKey::for_repository(root.path()).unwrap();
        let registry = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        registry
            .execute(
                "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
                [scope.as_str()],
            )
            .unwrap();
        if let Some(given_at) = link {
            registry
                .execute(
                    "INSERT INTO pull_request_links
                         (repository_id, kind, item_ulid, source, target, forge, url, given_at)
                     SELECT id, 'document', ?1, ?2, 'main', 'github', 'https://forge.example/open', ?3
                       FROM repositories",
                    rusqlite::params![NOTE, format!("manyhands/document/{NOTE}"), given_at],
                )
                .unwrap();
        }
        Self {
            _data: data,
            root,
            service,
            context,
        }
    }

    fn target() -> SynchronizationTarget {
        SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: NOTE.parse().unwrap(),
        }
    }

    fn binding(&self) -> SyncBinding {
        SyncBinding {
            root: self.root.path().to_owned(),
            target: Self::target(),
            republish: None,
        }
    }

    /// What `item sync` answers for `outcome`, and the document's link row,
    /// as `(link, given_at)`, afterwards.
    fn finished(
        &self,
        outcome: SynchronizationOutcome,
    ) -> (Ran, Option<(crate::forge::PullRequestLink, i64)>) {
        let accepted = accepted_in(self.root.path());
        let ran = self.binding().finished(
            &self.service,
            &accepted,
            &SynchronizationResult::Complete(outcome),
        );
        let row = self
            .service
            .read_session(RepositoryOperation::Read, |db| {
                let id: i64 = db.query_row("SELECT id FROM repositories", [], |row| row.get(0))?;
                let link =
                    crate::repository::pull_request_links::link_of(db, id, "document", NOTE)?;
                let given_at = link
                    .as_ref()
                    .map(|_| {
                        db.query_row(
                            "SELECT given_at FROM pull_request_links
                              WHERE repository_id = ?1 AND kind = 'document'",
                            [id],
                            |row| row.get(0),
                        )
                    })
                    .transpose()?;
                Ok(link.zip(given_at))
            })
            .unwrap();
        (ran, row)
    }

    fn published(&self) -> SynchronizationOutcome {
        SynchronizationOutcome::Published {
            target: Self::target(),
            oid: self.context,
        }
    }

    fn current(&self) -> SynchronizationOutcome {
        SynchronizationOutcome::AlreadyCurrent {
            target: Self::target(),
            oid: self.context,
        }
    }
}

/// `item sync` of a document in a pull-request repository that publishes, or
/// finds current, its branch answers with the link its row holds while the
/// primary does not hold the document, and writes nothing: the row is as the
/// promote left it, `given_at` included (Part F, R-10). Every other answer is
/// as it was: no data, and no row made.
#[test]
fn item_sync_gives_an_outstanding_document_link() {
    // Published, and already current: the row's link, and the row untouched.
    for (what, code, outcome) in [
        (
            "published",
            ResultCode::Ok,
            Documented::published as fn(&Documented) -> _,
        ),
        ("current", ResultCode::AlreadyApplied, Documented::current),
    ] {
        let note = Documented::new(PULL_REQUEST, false, Some(7));
        let (ran, row) = note.finished(outcome(&note));
        assert_eq!(ran.answer.code, code, "{what}");
        let Some(MutationDataDto::PullRequestLink(data)) = &ran.answer.data else {
            panic!("{what}: {:?}", ran.answer.data)
        };
        assert_eq!(
            data.pull_request.source,
            format!("manyhands/document/{NOTE}"),
            "{what}"
        );
        assert_eq!(
            data.pull_request.url.as_deref(),
            Some("https://forge.example/open"),
            "{what}"
        );
        // The row is the link the answer shows, and `given_at` is the
        // promote's: item sync does not renew it.
        let (link, given_at) = row.expect("the row stays");
        assert_eq!(
            crate::repository::PullRequestDto::from(&link),
            data.pull_request,
            "{what}"
        );
        assert_eq!(given_at, 7, "{what}");
    }

    // No row, the primary holding the document, and a direct repository give
    // today's answer: no data. None makes a row or changes one.
    let anyone = |what: &str, note: Documented, given: Option<i64>| {
        let (ran, row) = note.finished(note.published());
        assert!(ran.answer.data.is_none(), "{what}: {:?}", ran.answer.data);
        assert_eq!(row.map(|(_, given_at)| given_at), given, "{what}");
    };
    // No row, no proof: a document that was never promoted pays nothing for
    // the full content proof on every sync (decision 9). The count is the
    // thread's, and each case reads it before and after.
    let proofs = crate::repository::promotion::observe::document_proofs_run();
    anyone("no row", Documented::new(PULL_REQUEST, false, None), None);
    assert_eq!(
        crate::repository::promotion::observe::document_proofs_run(),
        proofs,
        "no row, no proof"
    );
    anyone(
        "the primary holds the document",
        Documented::new(PULL_REQUEST, true, Some(7)),
        Some(7),
    );
    assert!(
        crate::repository::promotion::observe::document_proofs_run() > proofs,
        "a row runs the proof: the count sees it"
    );
    anyone(
        "a direct repository",
        Documented::new(DIRECT, false, Some(7)),
        Some(7),
    );
}
