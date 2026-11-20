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
