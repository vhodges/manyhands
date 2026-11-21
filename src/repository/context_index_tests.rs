//! Each context's index rows, stored from that context's own observation by
//! `persist_context_index`, through the refresh and the rebuild that store
//! them (the fast-writes design, "Phase 1: the index").

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::{Oid, Repository};
use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

use super::*;
use crate::repository::{
    basis::{Basis, BasisCheck, check, settles},
    discovery::{list_contexts, stat_basis},
    git_fixtures::active_worktree_at,
};

/// A ticket on the primary, edited in its own worktree.
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
/// A directory under the worktree base, named for an item, that is no
/// worktree of the repository: an invalid item context.
const NOT_A_WORKTREE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
/// When the ticket was committed on the primary.
const TICKET_COMMITTED_AT: i64 = 1_700_000_100;

fn ticket_path() -> String {
    format!(".manyhands/tickets/{TICKET}/ticket.md")
}

/// A file in the ticket's comment directory that is not a comment: on the
/// primary, a problem under the ticket's directories.
fn not_a_comment_path() -> String {
    format!(".manyhands/comments/{TICKET}/notes.md")
}

fn ticket_source(title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\ntitle: {title}\n\
         type: task\nstatus: open\n---\nBody.\n"
    )
}

fn write(directory: &Path, path: &str, text: &str) {
    let path = directory.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Commits `paths` as they are on disk, on the checked-out branch, at `at`.
fn commit(repository: &Repository, paths: &[&str], at: i64) -> Oid {
    let mut index = repository.index().unwrap();
    for path in paths {
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::new(
        "Fixture",
        "fixture@example.invalid",
        &git2::Time::new(at, 0),
    )
    .unwrap();
    let parent = repository.head().ok().and_then(|h| h.peel_to_commit().ok());
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "fixture",
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
}

fn now_ns() -> i64 {
    i64::try_from(OffsetDateTime::now_utc().unix_timestamp_nanos()).unwrap()
}

fn text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

/// An enabled repository whose primary, `main`, holds the ticket, committed,
/// with a file beside it that is not a comment; the ticket's worktree, which
/// edits its title; and `NOT_A_WORKTREE`. Nothing is indexed since enabling.
struct Fixture {
    _directory: tempfile::TempDir,
    data: tempfile::TempDir,
    root: PathBuf,
    repository: Repository,
    service: RepositoryService,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let root = directory.path().to_owned();
        let data = tempfile::tempdir().unwrap();
        let repository = Repository::init(&root).unwrap();
        repository.set_head("refs/heads/main").unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        write(&root, "fixture.txt", "fixture\n");
        commit(&repository, &["fixture.txt"], 1_700_000_000);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        write(&root, &ticket_path(), &ticket_source("Primary copy"));
        // Enabling leaves its configuration for the caller to commit.
        commit(
            &repository,
            &[canonical::CONFIG_PATH, &ticket_path()],
            TICKET_COMMITTED_AT,
        );
        write(
            &root,
            &not_a_comment_path(),
            "---\nmanyhands_managed: true\n---\n",
        );
        let worktree = active_worktree_at(&repository, &root, "ticket", TICKET, TICKET);
        write(&worktree, &ticket_path(), &ticket_source("Worktree copy"));
        fs::create_dir_all(root.join(".manyhands/worktrees").join(NOT_A_WORKTREE)).unwrap();
        Self {
            _directory: directory,
            data,
            root,
            repository,
            service,
        }
    }

    fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    fn index(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    fn refresh(&self) {
        let outcome = self
            .service
            .refresh_repository(RefreshRepositoryRequest {
                root: self.root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(outcome, RefreshOutcome::Refreshed { .. }),
            "{outcome:?}"
        );
    }

    fn rebuild(&self) {
        self.service
            .rebuild_repository(RebuildRepositoryRequest {
                root: self.root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap();
    }

    /// Each stored context: its worktree, kind and item, in worktree order.
    fn contexts(&self) -> Vec<(String, String, Option<String>)> {
        self.index()
            .prepare("SELECT worktree_path, kind, item_id FROM contexts ORDER BY worktree_path")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// The items the context at `worktree` holds: ID and title.
    fn items(&self, worktree: &Path) -> Vec<(String, String)> {
        self.index()
            .prepare(
                "SELECT discovered_items.item_id, discovered_items.title
                   FROM discovered_items JOIN contexts ON contexts.id = discovered_items.context_id
                  WHERE contexts.worktree_path = ?1 ORDER BY discovered_items.item_id",
            )
            .unwrap()
            .query_map([text(worktree)], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// The problems stored for the context at `worktree`: path and code.
    fn problems(&self, worktree: &Path) -> Vec<(Option<String>, String)> {
        self.index()
            .prepare(
                "SELECT problems.path, problems.code
                   FROM problems JOIN contexts ON contexts.id = problems.context_id
                  WHERE contexts.worktree_path = ?1 ORDER BY problems.path, problems.code",
            )
            .unwrap()
            .query_map([text(worktree)], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// What the row of the context at `worktree` records of its indexing:
    /// its mark, basis, indexed time and activity base.
    fn indexing(&self, worktree: &Path) -> (i64, Option<String>, Option<i64>, Option<String>) {
        self.index()
            .query_row(
                "SELECT dirty, basis, indexed_at, activity_base FROM contexts
                  WHERE worktree_path = ?1",
                [text(worktree)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
    }

    /// The commit time the stored activity map holds for `path`.
    fn stored_activity(&self, path: &str) -> Option<i64> {
        self.index()
            .query_row(
                "SELECT committed_at FROM primary_activity WHERE path = ?1",
                [path],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    /// The repository's `refresh_required`, and each stored context's dirty
    /// mark by worktree, in worktree order.
    fn marks(&self) -> (bool, Vec<(String, i64)>) {
        let index = self.index();
        let behind = index
            .query_row("SELECT refresh_required FROM repositories", [], |row| {
                row.get(0)
            })
            .unwrap();
        let marks = index
            .prepare("SELECT worktree_path, dirty FROM contexts ORDER BY worktree_path")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        (behind, marks)
    }

    /// What `marks` reads once every context is stored, with the repository
    /// marked behind when `behind` and only the contexts at `dirty` dirty.
    fn stored_marks(&self, behind: bool, dirty: &[&Path]) -> (bool, Vec<(String, i64)>) {
        let marks = [
            self.root.clone(),
            self.worktree(TICKET),
            self.worktree(NOT_A_WORKTREE),
        ]
        .iter()
        .map(|worktree| {
            (
                text(worktree),
                i64::from(dirty.contains(&worktree.as_path())),
            )
        })
        .collect();
        (behind, marks)
    }

    /// Marks the context at `worktree` dirty, as a write that changed it does.
    fn mark(&self, worktree: &Path) {
        self.index()
            .execute(
                "UPDATE contexts SET dirty = 1 WHERE worktree_path = ?1",
                [text(worktree)],
            )
            .unwrap();
    }

    fn status(&self) -> IndexStatusDto {
        let repo = self.service.resolve_repository(&self.root).unwrap();
        self.service.index_status(&repo).unwrap()
    }

    /// The index state the repository list reports for the repository.
    fn listed(&self) -> IndexState {
        let [summary] = &self.service.list_repositories().unwrap().items[..] else {
            panic!("one registration");
        };
        summary.index.state
    }

    /// Whether the snapshot of the repository says its index is behind.
    fn snapshot_behind(&self) -> bool {
        self.service
            .repository_snapshot(&self.root)
            .unwrap()
            .refresh_required
    }

    /// The activity time the primary's copy of the ticket was stored with.
    fn primary_activity_at(&self) -> i64 {
        self.index()
            .query_row(
                "SELECT discovered_items.activity_at
                   FROM discovered_items JOIN contexts ON contexts.id = discovered_items.context_id
                  WHERE contexts.worktree_path = ?1 AND discovered_items.item_id = ?2",
                params![text(&self.root), TICKET],
                |row| row.get(0),
            )
            .unwrap()
    }
}

/// Whatever stores the index, each context's rows are its own
/// observation's. The primary holds its copy of the ticket its worktree
/// edits, and its problem under that ticket's directories, which the reads
/// leave out (Table A). The directory that is no worktree is a context of
/// its own, kind `invalid`, holding nothing but its problem, at the worktree
/// itself: the empty path, which the reads present at
/// `.manyhands/worktrees/<id>` in the primary's frame, as today.
#[test]
fn a_store_writes_the_primary_unhidden_and_an_invalid_context_as_its_own_row() {
    /// A store by name, and what runs it.
    type Store = (&'static str, fn(&Fixture));
    let stores: [Store; 2] = [("refresh", Fixture::refresh), ("rebuild", Fixture::rebuild)];
    for (store, run) in stores {
        let fixture = Fixture::new();
        run(&fixture);
        let root = &fixture.root;
        let edited = fixture.worktree(TICKET);
        let invalid = fixture.worktree(NOT_A_WORKTREE);

        assert_eq!(
            fixture.contexts(),
            [
                (text(root), "primary".to_owned(), None),
                (text(&edited), "active".to_owned(), Some(TICKET.to_owned())),
                (
                    text(&invalid),
                    "invalid".to_owned(),
                    Some(NOT_A_WORKTREE.to_owned())
                ),
            ],
            "{store}"
        );
        assert_eq!(
            fixture.items(root),
            [(TICKET.to_owned(), "Primary copy".to_owned())],
            "{store}"
        );
        assert_eq!(
            fixture.problems(root),
            [(Some(not_a_comment_path()), "invalid-path".to_owned())],
            "{store}"
        );
        assert_eq!(
            fixture.items(&edited),
            [(TICKET.to_owned(), "Worktree copy".to_owned())],
            "{store}"
        );
        assert_eq!(fixture.problems(&edited), [], "{store}");
        assert_eq!(fixture.items(&invalid), [], "{store}");
        assert_eq!(
            fixture.problems(&invalid),
            [(Some(String::new()), "context".to_owned())],
            "{store}"
        );
        let index = fixture.index();
        let unattached: i64 = index
            .query_row(
                "SELECT COUNT(*) FROM problems WHERE context_id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unattached, 0, "{store}");
        let configuration: String = index
            .query_row("SELECT state FROM configuration_observations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(configuration, "valid", "{store}");
    }
}

/// Indexing a context clears its mark and records what its observation rests
/// on: its basis, which a stat of the context now checks as the same, and the
/// time the observation began; for the primary also the head its activity
/// map is valid for, with the map. That map is loaded from the index for the
/// next observation, and stored again only when that observation changes it.
#[test]
fn indexing_a_context_clears_its_dirty_mark_and_stores_its_basis() {
    let fixture = Fixture::new();
    fixture.refresh();
    // What a registry an older build wrote holds once migrated: every
    // context owed an indexing, with nothing recorded of its last one.
    fixture
        .index()
        .execute(
            "UPDATE contexts SET dirty = 1, basis = NULL, indexed_at = NULL, activity_base = NULL",
            [],
        )
        .unwrap();
    let began = now_ns();
    fixture.refresh();
    let ended = now_ns();

    let listed = list_contexts(&fixture.repository, &fixture.root).unwrap();
    assert_eq!(listed.len(), 3);
    for context in &listed {
        let (dirty, basis, indexed_at, _) = fixture.indexing(&context.worktree);
        assert_eq!(dirty, 0, "{context:?}");
        let indexed_at = indexed_at.expect("an indexed time");
        assert!(
            (began..=ended).contains(&indexed_at),
            "{context:?}: {indexed_at} not in {began}..={ended}"
        );
        let basis = Basis::decode(&basis.expect("a basis")).expect("a basis this build reads");
        let now = stat_basis(
            &fixture.repository,
            &fixture.root,
            context,
            Some(&basis),
            indexed_at,
        );
        let same = match check(&basis, &now, indexed_at) {
            BasisCheck::Same => true,
            BasisCheck::Racy(files) => settles(&basis, &context.worktree, &files),
            BasisCheck::Moved => false,
        };
        assert!(same, "{context:?}: the stored basis is not the context's");
    }

    // The primary's row records the head its map is for; an item context's
    // records none.
    let head = fixture.repository.head().unwrap().target().unwrap();
    assert_eq!(fixture.indexing(&fixture.root).3, Some(head.to_string()));
    for id in [TICKET, NOT_A_WORKTREE] {
        assert_eq!(fixture.indexing(&fixture.worktree(id)).3, None, "{id}");
    }
    assert_eq!(
        fixture.stored_activity(&ticket_path()),
        Some(TICKET_COMMITTED_AT)
    );
    assert_eq!(fixture.primary_activity_at(), TICKET_COMMITTED_AT);

    // Every write of the map is counted from here on, and the time the map
    // holds for the ticket is changed by hand: an observation that reads it
    // from the index takes it from there, and one that builds the map anew
    // does not.
    fixture
        .index()
        .execute_batch(&format!(
            "CREATE TABLE activity_writes (path TEXT NOT NULL);
             CREATE TRIGGER activity_written AFTER INSERT ON primary_activity
             BEGIN INSERT INTO activity_writes (path) VALUES (NEW.path); END;
             UPDATE primary_activity SET committed_at = 1600000000 WHERE path = '{}';",
            ticket_path()
        ))
        .unwrap();
    let writes = || -> i64 {
        fixture
            .index()
            .query_row("SELECT COUNT(*) FROM activity_writes", [], |row| row.get(0))
            .unwrap()
    };
    // A refresh observes the primary only when it is owed an indexing or
    // moved: marked, it is observed again at the same head.
    fixture.mark(&fixture.root);
    fixture.refresh();
    assert_eq!(fixture.primary_activity_at(), 1_600_000_000);
    assert_eq!(writes(), 0, "the map is unchanged at the same head");

    // A commit on the primary advances the loaded map over it, which is
    // stored again with its new head.
    write(&fixture.root, "fixture.txt", "changed\n");
    let next = commit(&fixture.repository, &["fixture.txt"], 1_700_000_200);
    fixture.refresh();
    assert!(writes() > 0, "the advanced map is stored");
    assert_eq!(fixture.indexing(&fixture.root).3, Some(next.to_string()));
    assert_eq!(fixture.stored_activity(&ticket_path()), Some(1_600_000_000));
    assert_eq!(fixture.primary_activity_at(), 1_600_000_000);
}

/// A refresh or a rebuild that finds a context changing while it observes
/// it stores nothing of that context, which keeps its rows, marked dirty. A
/// refresh leaves every other context, which did not change, as it was
/// stored, with its own indexed time, and marks only that one: the
/// repository is not marked behind as a whole. A rebuild stores nothing, so
/// the others keep their rows and their marks, and it marks the repository
/// behind, which only a rebuild or refresh that completes clears.
#[test]
fn a_refresh_or_a_rebuild_that_races_an_edit_marks_only_the_raced_context_dirty() {
    for store in ["refresh", "rebuild"] {
        let fixture = Fixture::new();
        fixture.refresh();
        let edited = fixture.worktree(TICKET);
        let others = [fixture.root.clone(), fixture.worktree(NOT_A_WORKTREE)];
        let stored_at = others.clone().map(|other| fixture.indexing(&other).2);
        let raced = edited.clone();
        // Written again as it is, the context has moved since it was stored,
        // so a refresh observes it before the hook races it.
        let file = edited.join(ticket_path());
        fs::write(&file, fs::read(&file).unwrap()).unwrap();
        fixture.service.set_observation_hook_for_testing(move || {
            write(&raced, &ticket_path(), &ticket_source("Raced copy"));
        });
        if store == "refresh" {
            let outcome = fixture
                .service
                .refresh_repository(RefreshRepositoryRequest {
                    root: fixture.root.clone(),
                    operation_id: OperationId::new(),
                })
                .unwrap();
            assert!(
                matches!(&outcome, RefreshOutcome::RetryRequired { context: Some(context), .. }
                    if *context == edited),
                "{outcome:?}"
            );
        } else {
            fixture.rebuild();
        }

        assert_eq!(
            fixture.items(&edited),
            [(TICKET.to_owned(), "Worktree copy".to_owned())],
            "{store}"
        );
        assert_eq!(
            fixture.marks(),
            fixture.stored_marks(store == "rebuild", &[&edited]),
            "{store}"
        );
        for (other, stored_at) in others.iter().zip(stored_at) {
            assert_eq!(fixture.indexing(other).2, stored_at, "{store} {other:?}");
        }
        assert_eq!(
            fixture.items(&fixture.root),
            [(TICKET.to_owned(), "Primary copy".to_owned())],
            "{store}"
        );
    }
}

/// A checkpoint marks only the context it committed to as owed an indexing,
/// after its commit: the primary, the other contexts and the repository are
/// left as they were stored. One that commits nothing marks it all the same.
/// One whose mark cannot be written answers `RefreshPending` with its
/// commit, having marked nothing, and its retry marks without committing
/// again. A context the index stores nothing for has no mark to take: the
/// repository is marked behind as a whole instead, as before.
#[test]
fn a_checkpoint_marks_only_its_context_dirty() {
    let fixture = Fixture::new();
    fixture.refresh();
    let edited = fixture.worktree(TICKET);
    let context = ItemContext {
        root: fixture.root.clone(),
        kind: AuthoringKind::Ticket,
        item_id: TICKET.parse().unwrap(),
        branch: format!("manyhands/ticket/{TICKET}"),
        worktree: edited.clone(),
    };
    let worktree = Repository::open(&edited).unwrap();
    let path = PathBuf::from(ticket_path());
    let checkpoint = || {
        fixture
            .service
            .checkpoint_owned_paths(
                &worktree,
                &context,
                &CommitIdentity {
                    name: "Fixture".into(),
                    email: "fixture@example.invalid".into(),
                },
                CheckpointSpec {
                    operation: RepositoryOperation::SaveTicket,
                    subject: "Save ticket",
                    added_paths: &[path.as_path()],
                    removed_source: None,
                },
            )
            .unwrap()
    };
    let head = || worktree.head().unwrap().target().unwrap();
    let only_its_own = fixture.stored_marks(false, &[&edited]);

    let LocalCheckpoint::Checkpointed { commit_oid } = checkpoint() else {
        panic!("the worktree's edit is committed");
    };
    assert_eq!(head(), commit_oid);
    assert_eq!(fixture.marks(), only_its_own);
    fixture.refresh();
    assert!(matches!(checkpoint(), LocalCheckpoint::NoChange));
    assert_eq!(fixture.marks(), only_its_own);

    fixture.refresh();
    write(&edited, &ticket_path(), &ticket_source("Edited again"));
    *fixture.service.failure_point.lock().unwrap() = Some(FailurePoint::BeforeRegistryWrite);
    let LocalCheckpoint::RefreshPending { commit_oid } = checkpoint() else {
        panic!("a mark that cannot be written leaves the commit pending its indexing");
    };
    assert_eq!(head(), commit_oid);
    assert_eq!(fixture.marks(), fixture.stored_marks(false, &[]));
    assert!(matches!(checkpoint(), LocalCheckpoint::NoChange));
    assert_eq!(head(), commit_oid);
    assert_eq!(fixture.marks(), only_its_own);

    fixture.refresh();
    fixture
        .index()
        .execute(
            "DELETE FROM contexts WHERE worktree_path = ?1",
            [text(&edited)],
        )
        .unwrap();
    assert!(matches!(checkpoint(), LocalCheckpoint::NoChange));
    assert_eq!(
        fixture.marks(),
        (
            true,
            vec![
                (text(&fixture.root), 0),
                (text(&fixture.worktree(NOT_A_WORKTREE)), 0)
            ]
        )
    );
}

/// A context a pending row owes an indexing reads as dirty whatever its own
/// mark says. The row records the contexts it owes before its commit, as
/// `owed_contexts`, a JSON array of context keys, so a crash between that
/// commit and the mark leaves no stale row reading current. What this build
/// cannot read of it is owed in full; a row that completed owes nothing.
#[test]
fn a_context_owed_by_a_pending_row_reads_dirty() {
    let fixture = Fixture::new();
    fixture.refresh();
    let ticket: canonical::ItemId = TICKET.parse().unwrap();
    let other: canonical::ItemId = NOT_A_WORKTREE.parse().unwrap();
    // What a row owes as stored, its state, and whether the primary and the
    // ticket's context then read dirty.
    for (owed, state, primary, own) in [
        (Some(format!("[\"{TICKET}\"]")), "authoring", false, true),
        (
            Some("[\"primary\"]".to_owned()),
            "authoritative_observed",
            true,
            false,
        ),
        (
            Some(format!("[\"primary\",\"{TICKET}\"]")),
            "indexing",
            true,
            true,
        ),
        (Some("[]".to_owned()), "authoring", false, false),
        (None, "authoring", false, false),
        (Some(format!("[\"{TICKET}\"]")), "completed", false, false),
        (Some("primary".to_owned()), "authoring", true, true),
        (
            Some(format!("[\"{}\"]", TICKET.to_lowercase())),
            "authoring",
            true,
            true,
        ),
        (Some("[1]".to_owned()), "authoring", true, true),
        (
            Some("{\"primary\":true}".to_owned()),
            "authoring",
            true,
            true,
        ),
        (Some(String::new()), "authoring", true, true),
    ] {
        let case = format!("{owed:?} {state}");
        fixture
            .index()
            .execute(
                "INSERT INTO operation_records (
                    repository_id, root_path, operation_ulid, action, state, observed_at,
                    owed_contexts
                 ) SELECT id, root_path, ?1, 'save_ticket', ?2, 1, ?3 FROM repositories",
                params![OperationId::new().to_string(), state, owed],
            )
            .unwrap();

        assert_eq!(fixture.marks(), fixture.stored_marks(false, &[]), "{case}");
        // A close of the ticket changes the primary and the ticket's own
        // context; one of the other item, the primary and that item's.
        assert_eq!(
            fixture.service.lifecycle_indexed(&fixture.root, &ticket),
            !(primary || own),
            "{case}"
        );
        assert_eq!(
            fixture.service.lifecycle_indexed(&fixture.root, &other),
            !primary,
            "{case}"
        );
        assert_eq!(fixture.snapshot_behind(), primary || own, "{case}");
        assert_eq!(
            fixture.status().state,
            if primary || own {
                IndexStatusState::Stale
            } else {
                IndexStatusState::Current
            },
            "{case}"
        );
        fixture
            .index()
            .execute(
                "DELETE FROM operation_records WHERE action = 'save_ticket'",
                [],
            )
            .unwrap();
    }
}

/// `index status`, the repository list and the snapshot read the index as
/// stale while any context is dirty, whether the primary, an item's or one
/// that is no valid item context, though the repository is not marked
/// behind as a whole; `refreshed_at` stays when the last whole refresh
/// began. The next refresh stores the context, and the index reads current.
#[test]
fn index_status_is_stale_while_any_context_is_dirty() {
    let fixture = Fixture::new();
    fixture.refresh();
    for dirty in [
        fixture.root.clone(),
        fixture.worktree(TICKET),
        fixture.worktree(NOT_A_WORKTREE),
    ] {
        let current = fixture.status();
        assert_eq!(current.state, IndexStatusState::Current, "{dirty:?}");
        assert_eq!(fixture.listed(), IndexState::Current, "{dirty:?}");

        fixture.mark(&dirty);
        assert_eq!(fixture.marks(), fixture.stored_marks(false, &[&dirty]));
        let status = fixture.status();
        assert_eq!(status.state, IndexStatusState::Stale, "{dirty:?}");
        assert_eq!(status.refreshed_at, current.refreshed_at, "{dirty:?}");
        assert_eq!(fixture.listed(), IndexState::Stale, "{dirty:?}");
        assert!(fixture.snapshot_behind(), "{dirty:?}");

        fixture.refresh();
        assert_eq!(fixture.marks(), fixture.stored_marks(false, &[]));
        assert!(!fixture.snapshot_behind(), "{dirty:?}");
    }
}

/// A close or a promotion of an item changes the primary and the item's own
/// context, and reads the marks of those two to answer `discovery: current`:
/// not while either is dirty, or the repository is marked behind as a whole.
/// Another item's mark does not hold it back, though the snapshot then reads
/// the index as behind. Its writes mark the same two.
#[test]
fn the_lifecycle_reads_the_dirty_marks_of_the_contexts_it_changed() {
    let fixture = Fixture::new();
    fixture.refresh();
    let ticket: canonical::ItemId = TICKET.parse().unwrap();
    let own = fixture.worktree(TICKET);
    let another = fixture.worktree(NOT_A_WORKTREE);
    let indexed = || fixture.service.lifecycle_indexed(&fixture.root, &ticket);
    assert!(indexed());
    // What is marked (the whole repository when none), and whether the
    // lifecycle of the ticket then reads current.
    for (marked, current) in [
        (Some(&another), true),
        (Some(&own), false),
        (Some(&fixture.root), false),
        (None, false),
    ] {
        match marked {
            Some(worktree) => fixture.mark(worktree),
            None => {
                fixture
                    .index()
                    .execute("UPDATE repositories SET refresh_required = 1", [])
                    .unwrap();
            }
        }
        assert_eq!(indexed(), current, "{marked:?}");
        assert!(fixture.snapshot_behind(), "{marked:?}");
        fixture.refresh();
        assert!(indexed(), "{marked:?}");
    }

    mark_contexts_dirty(
        &fixture.service.registry_path,
        &fixture.root,
        &lifecycle_contexts(&ticket),
        RepositoryOperation::RepositorySnapshot,
    )
    .unwrap();
    assert_eq!(
        fixture.marks(),
        fixture.stored_marks(false, &[&fixture.root, &own])
    );
    assert!(!indexed());
}
