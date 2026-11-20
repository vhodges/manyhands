//! The situations of Table A in the fast-writes design
//! (`docs/plans/2026-11-19-fast-writes-design.md`): which copy of an item
//! the reads present once each context's index rows come only from that
//! context.
//!
//! Each world is made with the real service, then stored as the new design
//! stores it: the primary's rows hold every item of the root, an item that
//! has a valid worktree among them, and a worktree that an outside change
//! made invalid is a context row of its own, with the kind `invalid` and its
//! problems. Where today's refresh stores the old shape, the helpers here
//! write what the new store adds; where the index already holds it, they
//! write nothing. So a world reads the same before and after the store
//! changes, and a row that is to pass only once the store writes the new
//! shape itself makes its world without them.
//!
//! The read targets run their rows through `run`, which runs every row and
//! names each that fails.

use std::{
    fs,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
};

use git2::{Repository, WorktreePruneOptions};
use manyhands::repository::{
    AuthoringKind, AuthoringTarget, ContextIntent, ContextProvisionOutcome, ContextReleaseOutcome,
    ExpectedPathObservation, ReleaseContextRequest, RepositoryService, ResolvedRepository,
    SaveOutcome, SaveTicketRequest, TicketDraft,
};
use rusqlite::{Connection, params};

use super::{
    EnabledRepository, TestRepository,
    items::{
        COMMENT_A, COMMENT_B, COMMENT_C, COMMENT_D, COMMITTED_AT, DOCUMENT_A, DOCUMENT_B,
        RELATED_A, RELATED_B, RELATED_C, RELATED_D, SHARED_SLUG, TICKET_A, TICKET_B, TICKET_C,
        comment_path, commit, context_worktree, create_document_context, document_source, index,
        item_id, refresh_completely, ticket_path, ticket_source, write, write_comment,
    },
};

/// A ticket merged to the primary, with a worktree for its first edit.
pub const EDITED: &str = TICKET_A;
/// A ticket only on the primary.
pub const PRIMARY_ONLY: &str = TICKET_B;
/// A ticket with a worktree, whose primary copy no longer parses.
pub const BROKEN_ON_PRIMARY: &str = TICKET_C;
/// A document merged to the primary, with a worktree for its first edit.
pub const EDITED_DOCUMENT: &str = DOCUMENT_A;
pub const EDITED_DOCUMENT_PATH: &str = "docs/edited.md";
/// A ticket and a document created in their worktrees and on no primary.
pub const NEW_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";
pub const NEW_DOCUMENT: &str = DOCUMENT_B;
pub const NEW_DOCUMENT_PATH: &str = "docs/new.md";

/// The title of each copy of an edited item.
pub const PRIMARY_TITLE: &str = "Primary copy";
pub const WORKTREE_TITLE: &str = "Worktree copy";
pub const PRIMARY_ONLY_TITLE: &str = "Only on the primary";
pub const NEW_TICKET_TITLE: &str = "New in its worktree";

/// A comment the edited ticket's worktree has and the primary does not.
pub const WORKTREE_COMMENT: &str = COMMENT_A;
/// A comment the primary has on the edited ticket and its worktree does not.
pub const PRIMARY_COMMENT: &str = COMMENT_B;
/// A comment committed on the primary, which every checkout then holds.
pub const COMMITTED_COMMENT: &str = COMMENT_C;
/// A comment on the new ticket, in its worktree.
pub const NEW_COMMENT: &str = COMMENT_D;
pub const CREATED_AT: &str = "2026-09-30T12:00:01Z";

/// The relationship world: the edited ticket's primary copy has the short
/// code another ticket also has, `RELATIONSHIP_PARENT` as its parent and a
/// dependency on `DEPENDENT`, which depends on it: a dependency cycle while
/// that copy counts. Its worktree copy has a short code of its own, no
/// parent and one dependency, on `WORKTREE_DEPENDENCY`.
pub const RELATIONSHIP_PARENT: &str = RELATED_A;
pub const DEPENDENT: &str = RELATED_B;
pub const WORKTREE_DEPENDENCY: &str = RELATED_C;
pub const SLUG_HOLDER: &str = RELATED_D;
pub const OLD_SLUG: &str = SHARED_SLUG;
pub const NEW_SLUG: &str = "zz-zz-zzzzz";

/// What an outside `git checkout` leaves an item worktree on.
pub const SCRATCH_BRANCH: &str = "scratch";

/// A file in an item's comment directory that is not a comment.
pub const NOT_A_COMMENT: &str = "---\nmanyhands_managed: true\n---\n";

pub fn not_a_comment_path(item: &str) -> String {
    format!(".manyhands/comments/{item}/notes.md")
}

/// The worktree's path relative to the root, as today's refresh stores a
/// problem about it in the primary context.
pub fn worktree_relative(id: &str) -> String {
    format!(".manyhands/worktrees/{id}")
}

/// One of the situations, refreshed and stored as the new design stores it.
pub struct World {
    pub fixture: TestRepository,
    pub enabled: EnabledRepository,
    pub repo: ResolvedRepository,
    /// The root as the index stores it.
    pub root: PathBuf,
}

impl World {
    pub fn service(&self) -> &RepositoryService {
        &self.enabled.service
    }

    pub fn root_text(&self) -> String {
        self.root.to_str().unwrap().to_owned()
    }

    /// Where the item's worktree is, or was.
    pub fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    pub fn worktree_text(&self, id: &str) -> String {
        self.worktree(id).to_str().unwrap().to_owned()
    }

    /// A connection to the index that enforces its foreign keys, so that a
    /// context's rows go with it.
    pub fn index(&self) -> Connection {
        let connection = index(self.enabled.data_directory.path());
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .unwrap();
        connection
    }

    fn repository_id(&self, connection: &Connection) -> i64 {
        connection
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap()
    }

    fn primary_context(&self, connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT id FROM contexts WHERE worktree_path = ?1",
                [self.root_text()],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn holds_context_at(&self, connection: &Connection, worktree: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM contexts WHERE worktree_path = ?1)",
                [worktree],
                |row| row.get(0),
            )
            .unwrap()
    }
}

fn enabled_world() -> (TestRepository, EnabledRepository) {
    let fixture = super::born_repository();
    let enabled = super::enabled_repository(&fixture);
    (fixture, enabled)
}

fn finish(fixture: TestRepository, enabled: EnabledRepository) -> World {
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let root = fs::canonicalize(&fixture.root).unwrap();
    World {
        fixture,
        enabled,
        repo,
        root,
    }
}

/// Commits the configuration enabling wrote with `paths`, so that the
/// items are on the primary and a worktree can be made to edit them.
fn commit_with_configuration(fixture: &TestRepository, paths: &[&str]) {
    let mut all = vec![".manyhands/config.toml"];
    all.extend_from_slice(paths);
    commit(fixture, &all, COMMITTED_AT);
}

/// Makes the worktree to edit an item the primary holds, as `context
/// prepare` does, and indexes it.
fn prepare_edit(service: &RepositoryService, root: &Path, kind: AuthoringKind, id: &str) {
    let outcome = service
        .prepare_context(AuthoringTarget {
            root: root.to_owned(),
            kind,
            item_id: item_id(id),
            intent: ContextIntent::Edit,
            operation_id: super::new_operation_id(),
        })
        .unwrap();
    assert!(
        matches!(outcome, ContextProvisionOutcome::Created(_)),
        "{id}: no worktree was made"
    );
}

/// Creates a ticket in a worktree of its own, as `ticket create` does.
fn create_ticket_in_worktree(service: &RepositoryService, root: &Path, id: &str, title: &str) {
    let outcome = service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: root.to_owned(),
                kind: AuthoringKind::Ticket,
                item_id: item_id(id),
                intent: ContextIntent::Create,
                operation_id: super::new_operation_id(),
            },
            draft: TicketDraft {
                title: title.to_owned(),
                ticket_type: "task".to_owned(),
                status: "open".to_owned(),
                project: None,
                team: None,
                body: format!("Body of {title}.\n"),
            },
            expected_path: ExpectedPathObservation::Missing,
        })
        .unwrap();
    assert!(
        matches!(outcome, SaveOutcome::Saved { .. }),
        "{id}: not saved"
    );
}

/// An outside `git checkout` in the item's worktree: another branch, at the
/// same commit, so the worktree is no longer a valid item context.
fn check_out_another_branch(worktree: &Path) {
    let repository = Repository::open(worktree).unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    repository.branch(SCRATCH_BRANCH, &head, false).unwrap();
    repository
        .set_head(&format!("refs/heads/{SCRATCH_BRANCH}"))
        .unwrap();
}

/// The primary's copy of an item that has a worktree, with its edges and
/// its comments, as the new design's store writes it: today's refresh
/// leaves it out. Nothing is written when the primary already holds it.
pub struct PrimaryCopy<'a> {
    pub id: &'a str,
    pub kind: AuthoringKind,
    pub path: String,
    pub title: &'a str,
    pub slug: Option<&'a str>,
    pub parent: Option<&'a str>,
    pub deps: &'a [&'a str],
    pub comments: &'a [&'a str],
}

impl<'a> PrimaryCopy<'a> {
    pub fn ticket(id: &'a str, title: &'a str) -> Self {
        Self {
            id,
            kind: AuthoringKind::Ticket,
            path: ticket_path(id),
            title,
            slug: None,
            parent: None,
            deps: &[],
            comments: &[],
        }
    }
}

pub fn store_primary_copy(world: &World, copy: PrimaryCopy<'_>) {
    let connection = world.index();
    let context = world.primary_context(&connection);
    let held: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM discovered_items WHERE context_id = ?1 AND item_id = ?2)",
            params![context, copy.id],
            |row| row.get(0),
        )
        .unwrap();
    if held {
        return;
    }
    let (kind, ticket_type, status) = match copy.kind {
        AuthoringKind::Ticket => ("ticket", Some("task"), Some("open")),
        AuthoringKind::Document => ("document", None, None),
    };
    connection
        .execute(
            "INSERT INTO discovered_items
                (context_id, item_id, kind, canonical_path, title, ticket_type, status,
                 activity_at, activity_source, unknown_metadata, slug)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'git',
                     '{\"not_representable\":false,\"values\":{}}', ?9)",
            params![
                context,
                copy.id,
                kind,
                copy.path,
                copy.title,
                ticket_type,
                status,
                COMMITTED_AT,
                copy.slug
            ],
        )
        .unwrap();
    let row = connection.last_insert_rowid();
    let edges = copy
        .parent
        .iter()
        .map(|target| (*target, "parent"))
        .chain(copy.deps.iter().map(|target| (*target, "deps")));
    for (target, kind) in edges {
        connection
            .execute(
                "INSERT INTO item_edges (item_id, target_id, kind) VALUES (?1, ?2, ?3)",
                params![row, target, kind],
            )
            .unwrap();
    }
    for comment in copy.comments {
        connection
            .execute(
                "INSERT INTO discovered_comments (item_id, comment_id, canonical_path, created_at)
                 VALUES (?1, ?2, ?3, 0)",
                params![row, comment, comment_path(copy.id, comment)],
            )
            .unwrap();
    }
}

/// A problem the primary's observation finds under an item's directories,
/// which today's refresh leaves out when the item has a worktree. Nothing
/// is written when the primary already holds a problem at `path`.
pub fn store_primary_problem(world: &World, path: &str, code: &str) {
    let connection = world.index();
    let context = world.primary_context(&connection);
    let held: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM problems WHERE context_id = ?1 AND path = ?2)",
            params![context, path],
            |row| row.get(0),
        )
        .unwrap();
    if held {
        return;
    }
    connection
        .execute(
            "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
             VALUES (?1, ?2, ?3, ?4, 'as the primary observation finds it', 0)",
            params![world.repository_id(&connection), context, path, code],
        )
        .unwrap();
}

/// The item's broken worktree as a context of its own: kind `invalid`, the
/// item's ID, the branch an outside change left it on, and the problem
/// today's refresh stores in the primary context at the worktree's path,
/// attached to it instead, at the worktree itself (an empty path, as a
/// problem about a working tree is stored relative to it). Nothing is
/// written when the index already holds a context there.
pub fn store_invalid_context(world: &World, id: &str) {
    let connection = world.index();
    let worktree = world.worktree_text(id);
    if world.holds_context_at(&connection, &worktree) {
        return;
    }
    let repository = Repository::open(world.worktree(id)).unwrap();
    let head = repository.head().unwrap();
    connection
        .execute(
            "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id, head_oid)
             VALUES (?1, 'invalid', ?2, ?3, ?4, ?5)",
            params![
                world.repository_id(&connection),
                // A detached head is on no branch.
                head.is_branch().then(|| head.shorthand()).flatten(),
                worktree,
                id,
                head.target().map(|oid| oid.to_string())
            ],
        )
        .unwrap();
    let context = connection.last_insert_rowid();
    let moved = connection
        .execute(
            "UPDATE problems SET context_id = ?1, path = ''
              WHERE context_id = ?2 AND code = 'context' AND path = ?3",
            params![
                context,
                world.primary_context(&connection),
                worktree_relative(id)
            ],
        )
        .unwrap();
    assert_eq!(
        moved, 1,
        "today's refresh stores one problem at {id}'s worktree"
    );
}

/// A valid ticket worktree's own rows, as each context's rows come from its
/// own observation: the context and its one ticket. Nothing is written when
/// the index already holds a context there.
pub fn store_item_context(world: &World, id: &str, title: &str) {
    let connection = world.index();
    let worktree = world.worktree_text(id);
    if world.holds_context_at(&connection, &worktree) {
        return;
    }
    let head = Repository::open(world.worktree(id))
        .unwrap()
        .head()
        .unwrap()
        .target()
        .map(|oid| oid.to_string());
    connection
        .execute(
            "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id, head_oid)
             VALUES (?1, 'active', ?2, ?3, ?4, ?5)",
            params![
                world.repository_id(&connection),
                format!("manyhands/ticket/{id}"),
                worktree,
                id,
                head
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO discovered_items
                (context_id, item_id, kind, canonical_path, title, ticket_type, status,
                 activity_at, activity_source, unknown_metadata)
             VALUES (?1, ?2, 'ticket', ?3, ?4, 'task', 'open', ?5, 'git',
                     '{\"not_representable\":false,\"values\":{}}')",
            params![
                connection.last_insert_rowid(),
                id,
                ticket_path(id),
                title,
                COMMITTED_AT
            ],
        )
        .unwrap();
}

/// A primary with the edited ticket and a comment on it committed, and a
/// worktree made to edit it; refreshed.
fn edited_ticket() -> (TestRepository, EnabledRepository) {
    let (fixture, enabled) = enabled_world();
    let root = fixture.root.clone();
    write(
        &root,
        &ticket_path(EDITED),
        &ticket_source(EDITED, PRIMARY_TITLE, ""),
    );
    write_comment(&root, EDITED, COMMITTED_COMMENT, None, CREATED_AT, "");
    commit_with_configuration(
        &fixture,
        &[
            &ticket_path(EDITED),
            &comment_path(EDITED, COMMITTED_COMMENT),
        ],
    );
    // The primary's commit is made outside Manyhands: a refresh indexes it,
    // so the world does not rest on `context prepare` refreshing the whole
    // repository (after Task 10 it indexes only the context it makes).
    refresh_completely(&enabled.service, &root);
    prepare_edit(&enabled.service, &root, AuthoringKind::Ticket, EDITED);
    (fixture, enabled)
}

/// The primary copy of `edited_ticket`, as the new store holds it.
fn store_edited_primary_copy(world: &World) {
    store_primary_copy(
        world,
        PrimaryCopy {
            comments: &[COMMITTED_COMMENT],
            ..PrimaryCopy::ticket(EDITED, PRIMARY_TITLE)
        },
    );
}

/// Rows 1 and 2: first edits of merged items.
///
/// - `EDITED` is on the primary, with a comment there its worktree does not
///   have and a file in its comment directory that is not a comment. Its
///   worktree has an edit and a comment of its own.
/// - `EDITED_DOCUMENT` is on the primary and has an edit in its worktree.
/// - `BROKEN_ON_PRIMARY` has a worktree; its primary copy no longer parses.
/// - `PRIMARY_ONLY` has no worktree. It has the short code `SHARED_SLUG`,
///   a committed comment and a dependency on `EDITED`.
///
/// Nothing after the worktrees were made is committed.
pub fn first_edits() -> World {
    let (fixture, enabled) = enabled_world();
    let root = fixture.root.clone();
    write(
        &root,
        &ticket_path(EDITED),
        &ticket_source(EDITED, PRIMARY_TITLE, ""),
    );
    write(
        &root,
        &ticket_path(BROKEN_ON_PRIMARY),
        &ticket_source(BROKEN_ON_PRIMARY, "Broken on the primary", ""),
    );
    write(
        &root,
        EDITED_DOCUMENT_PATH,
        &document_source(EDITED_DOCUMENT, PRIMARY_TITLE, ""),
    );
    write(
        &root,
        &ticket_path(PRIMARY_ONLY),
        &ticket_source(
            PRIMARY_ONLY,
            PRIMARY_ONLY_TITLE,
            &format!("slug: {SHARED_SLUG}\ndeps: [{EDITED}]\n"),
        ),
    );
    write_comment(&root, PRIMARY_ONLY, COMMITTED_COMMENT, None, CREATED_AT, "");
    commit_with_configuration(
        &fixture,
        &[
            &ticket_path(EDITED),
            &ticket_path(BROKEN_ON_PRIMARY),
            EDITED_DOCUMENT_PATH,
            &ticket_path(PRIMARY_ONLY),
            &comment_path(PRIMARY_ONLY, COMMITTED_COMMENT),
        ],
    );
    for (kind, id) in [
        (AuthoringKind::Ticket, EDITED),
        (AuthoringKind::Ticket, BROKEN_ON_PRIMARY),
        (AuthoringKind::Document, EDITED_DOCUMENT),
    ] {
        prepare_edit(&enabled.service, &root, kind, id);
    }
    let worktree = |id: &str| context_worktree(&root, id);
    write(
        &worktree(EDITED),
        &ticket_path(EDITED),
        &ticket_source(EDITED, WORKTREE_TITLE, ""),
    );
    write_comment(
        &worktree(EDITED),
        EDITED,
        WORKTREE_COMMENT,
        None,
        CREATED_AT,
        "",
    );
    write(
        &worktree(EDITED_DOCUMENT),
        EDITED_DOCUMENT_PATH,
        &document_source(EDITED_DOCUMENT, WORKTREE_TITLE, ""),
    );
    write_comment(&root, EDITED, PRIMARY_COMMENT, None, CREATED_AT, "");
    write(&root, &not_a_comment_path(EDITED), NOT_A_COMMENT);
    write(&root, &ticket_path(BROKEN_ON_PRIMARY), "no front matter\n");
    refresh_completely(&enabled.service, &root);
    let world = finish(fixture, enabled);
    store_primary_copy(
        &world,
        PrimaryCopy {
            comments: &[PRIMARY_COMMENT],
            ..PrimaryCopy::ticket(EDITED, PRIMARY_TITLE)
        },
    );
    store_primary_problem(&world, &not_a_comment_path(EDITED), "invalid-path");
    store_primary_copy(
        &world,
        PrimaryCopy {
            kind: AuthoringKind::Document,
            path: EDITED_DOCUMENT_PATH.to_owned(),
            ..PrimaryCopy::ticket(EDITED_DOCUMENT, PRIMARY_TITLE)
        },
    );
    store_primary_problem(
        &world,
        &ticket_path(BROKEN_ON_PRIMARY),
        "missing-front-matter",
    );
    world
}

/// Row 7: the edited ticket's two copies relate to different tickets. See
/// `RELATIONSHIP_PARENT`.
pub fn relationships() -> World {
    let (fixture, enabled) = enabled_world();
    let root = fixture.root.clone();
    let tickets = [
        (
            RELATIONSHIP_PARENT,
            "Parent of the primary copy",
            String::new(),
        ),
        (
            DEPENDENT,
            "Depends on the edited ticket",
            format!("deps: [{EDITED}]\n"),
        ),
        (
            WORKTREE_DEPENDENCY,
            "Dependency of the worktree copy",
            String::new(),
        ),
        (
            SLUG_HOLDER,
            "Holds the old short code",
            format!("slug: {OLD_SLUG}\n"),
        ),
        (
            EDITED,
            PRIMARY_TITLE,
            format!("slug: {OLD_SLUG}\nparent: {RELATIONSHIP_PARENT}\ndeps: [{DEPENDENT}]\n"),
        ),
    ];
    let paths: Vec<String> = tickets.iter().map(|(id, _, _)| ticket_path(id)).collect();
    for (id, title, extra) in &tickets {
        write(&root, &ticket_path(id), &ticket_source(id, title, extra));
    }
    commit_with_configuration(
        &fixture,
        &paths.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    prepare_edit(&enabled.service, &root, AuthoringKind::Ticket, EDITED);
    write(
        &context_worktree(&root, EDITED),
        &ticket_path(EDITED),
        &ticket_source(
            EDITED,
            WORKTREE_TITLE,
            &format!("slug: {NEW_SLUG}\ndeps: [{WORKTREE_DEPENDENCY}]\n"),
        ),
    );
    refresh_completely(&enabled.service, &root);
    let world = finish(fixture, enabled);
    store_primary_copy(
        &world,
        PrimaryCopy {
            slug: Some(OLD_SLUG),
            parent: Some(RELATIONSHIP_PARENT),
            deps: &[DEPENDENT],
            ..PrimaryCopy::ticket(EDITED, PRIMARY_TITLE)
        },
    );
    world
}

/// Row 3: the edited ticket's worktree, which holds no work, released by
/// `context release`.
pub fn released() -> World {
    let (fixture, enabled) = edited_ticket();
    let world = finish(fixture, enabled);
    store_edited_primary_copy(&world);
    let outcome = world
        .service()
        .release_context(ReleaseContextRequest {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            item_id: item_id(EDITED),
            operation_id: super::new_operation_id(),
        })
        .unwrap();
    assert_eq!(outcome, ContextReleaseOutcome::Released);
    world
}

/// Row 3: the edited ticket's worktree removed, and its context's rows
/// deleted, as a command that removes a worktree does under the new
/// design: the primary is not indexed again. The primary copy is the one
/// the store wrote, so this world is made without `store_primary_copy`.
pub fn rows_deleted() -> World {
    let (fixture, enabled) = edited_ticket();
    let world = finish(fixture, enabled);
    world
        .fixture
        .repository
        .find_worktree(EDITED)
        .unwrap()
        .prune(Some(
            WorktreePruneOptions::new().valid(true).working_tree(true),
        ))
        .unwrap();
    assert!(!world.worktree(EDITED).exists());
    world
        .index()
        .execute(
            "DELETE FROM contexts WHERE worktree_path = ?1",
            [world.worktree_text(EDITED)],
        )
        .unwrap();
    world
}

/// Row 3: the edited ticket's worktree deleted outside Manyhands, and the
/// repository refreshed.
pub fn removed_outside() -> World {
    let (fixture, enabled) = edited_ticket();
    let world = finish(fixture, enabled);
    store_edited_primary_copy(&world);
    fs::remove_dir_all(world.worktree(EDITED)).unwrap();
    refresh_completely(world.service(), &world.root);
    world
}

/// Row 4: the edited ticket's worktree on another branch after an outside
/// `git checkout`, and the repository refreshed. With `store`, the broken
/// worktree is stored as the new design stores it; without, as the store
/// writes it.
pub fn invalid_worktree(store: bool) -> World {
    let (fixture, enabled) = edited_ticket();
    check_out_another_branch(&context_worktree(&fixture.root, EDITED));
    refresh_completely(&enabled.service, &fixture.root);
    let world = finish(fixture, enabled);
    if store {
        store_invalid_context(&world, EDITED);
    }
    world
}

/// Row 5: a ticket and a document created in their own worktrees, the
/// ticket with a comment there.
pub fn new_items_in_worktrees() -> World {
    let (fixture, enabled) = enabled_world();
    let root = fixture.root.clone();
    commit_with_configuration(&fixture, &[]);
    create_ticket_in_worktree(&enabled.service, &root, NEW_TICKET, NEW_TICKET_TITLE);
    create_document_context(&enabled.service, &root, NEW_DOCUMENT, NEW_DOCUMENT_PATH);
    write_comment(
        &context_worktree(&root, NEW_TICKET),
        NEW_TICKET,
        NEW_COMMENT,
        None,
        CREATED_AT,
        "",
    );
    refresh_completely(&enabled.service, &root);
    finish(fixture, enabled)
}

/// Row 6: the edited ticket, with an edit in its worktree, and a new ticket
/// in a worktree of its own; then the primary's configuration made invalid
/// and the repository refreshed. With `store`, each valid worktree's rows
/// are stored from its own observation, as the new design may store them;
/// without, as the store writes them.
pub fn invalid_configuration(store: bool) -> World {
    let (fixture, enabled) = edited_ticket();
    let root = fixture.root.clone();
    create_ticket_in_worktree(&enabled.service, &root, NEW_TICKET, NEW_TICKET_TITLE);
    write(
        &context_worktree(&root, EDITED),
        &ticket_path(EDITED),
        &ticket_source(EDITED, WORKTREE_TITLE, ""),
    );
    fs::write(root.join(".manyhands/config.toml"), "not = [valid").unwrap();
    refresh_completely(&enabled.service, &root);
    let world = finish(fixture, enabled);
    if store {
        store_item_context(&world, EDITED, WORKTREE_TITLE);
        store_item_context(&world, NEW_TICKET, NEW_TICKET_TITLE);
    }
    world
}

/// What a row needs to pass: the readers of Task 6 alone, because its
/// world is stored by the helpers here as the new design stores it, or
/// reads the same whatever the store writes; or the store of Task 7,
/// because it reads, or asserts, what only a store that writes each
/// context from its own observation leaves. Rows of the second kind are
/// marked `// needs Task 7 storage` where they are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Needs {
    Readers,
    Task7Storage,
}

/// One case of a Table A row in one read target.
pub struct Row {
    /// The Table A row, by the name the plan gives its test row.
    pub table_a: &'static str,
    /// What this case arranges.
    pub case: &'static str,
    pub needs: Needs,
    pub check: fn(),
}

/// Runs every row, then fails naming each row that failed and why, so that
/// one run says which rows hold.
pub fn run(rows: &[Row]) {
    let failed: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let payload = panic::catch_unwind(AssertUnwindSafe(row.check)).err()?;
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|text| (*text).to_owned())
                })
                .unwrap_or_default();
            Some(format!(
                "{} [{}] ({:?}): {message}",
                row.table_a, row.case, row.needs
            ))
        })
        .collect();
    assert!(
        failed.is_empty(),
        "{} of {} rows failed:\n{}",
        failed.len(),
        rows.len(),
        failed.join("\n\n")
    );
}
