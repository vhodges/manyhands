use std::collections::BTreeMap;

use git2::{Oid, Repository, Signature, Time};
use rusqlite::Connection;

use super::*;
use crate::repository::discovery::{migrate_registry, open_registry};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const NEW_DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
const OLD_DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
const CLOSED: &str = "closed_at: 2026-01-01T00:00:00Z\nclosed_by: Fixture\n";
const CLOSED_AGAIN: &str = "closed_at: 2026-02-02T00:00:00Z\nclosed_by: Fixture\n";

type Files = BTreeMap<String, String>;

fn ticket(id: &str, closed: &str, body: &str) -> String {
    let status = if closed.is_empty() { "open" } else { "closed" };
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: T\ntype: task\nstatus: {status}\n{closed}---\n{body}\n"
    )
}

fn document(id: &str, body: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: D\n---\n{body}\n"
    )
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// A repository whose commits a test writes by naming every file of the tree
/// and the committer time.
struct Fixture {
    _directory: tempfile::TempDir,
    repository: Repository,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::init(directory.path()).unwrap();
        Self {
            _directory: directory,
            repository,
        }
    }

    fn tree(&self, files: &Files) -> Oid {
        enum Node {
            Blob(Oid),
            Tree(BTreeMap<String, Node>),
        }
        fn write(repository: &Repository, directory: &BTreeMap<String, Node>) -> Oid {
            let mut builder = repository.treebuilder(None).unwrap();
            for (name, node) in directory {
                match node {
                    Node::Blob(oid) => builder.insert(name, *oid, 0o100644).unwrap(),
                    Node::Tree(inner) => builder
                        .insert(name, write(repository, inner), 0o040000)
                        .unwrap(),
                };
            }
            builder.write().unwrap()
        }
        let mut root = BTreeMap::new();
        for (path, content) in files {
            let blob = self.repository.blob(content.as_bytes()).unwrap();
            let mut directory = &mut root;
            let mut parts = path.split('/').peekable();
            while let Some(part) = parts.next() {
                if parts.peek().is_none() {
                    directory.insert(part.to_owned(), Node::Blob(blob));
                } else {
                    let Node::Tree(inner) = directory
                        .entry(part.to_owned())
                        .or_insert_with(|| Node::Tree(BTreeMap::new()))
                    else {
                        panic!("{path} is under a file");
                    };
                    directory = inner;
                }
            }
        }
        write(&self.repository, &root)
    }

    fn commit(&self, files: &Files, parents: &[Oid], seconds: i64) -> Oid {
        let tree = self.repository.find_tree(self.tree(files)).unwrap();
        let signature =
            Signature::new("Fixture", "fixture@example.invalid", &Time::new(seconds, 0)).unwrap();
        let parents: Vec<_> = parents
            .iter()
            .map(|oid| self.repository.find_commit(*oid).unwrap())
            .collect();
        let parents: Vec<_> = parents.iter().collect();
        self.repository
            .commit(None, &signature, &signature, "fixture", &tree, &parents)
            .unwrap()
    }
}

fn with(files: &Files, changes: &[(String, Option<String>)]) -> Files {
    let mut files = files.clone();
    for (path, content) in changes {
        match content {
            Some(content) => files.insert(path.clone(), content.clone()),
            None => files.remove(path),
        };
    }
    files
}

fn put(path: &str, content: String) -> (String, Option<String>) {
    (path.to_owned(), Some(content))
}

/// The files of the history every row starts from: an open ticket, a merged
/// document and a file the primary changes on its own.
fn base_files() -> Files {
    Files::from([
        ("README".into(), "one\n".into()),
        (ticket_path(TICKET), ticket(TICKET, "", "body")),
        ("docs/old.md".into(), document(OLD_DOC, "v1")),
    ])
}

#[derive(Clone, Copy, Debug)]
enum Style {
    MergeCommit,
    Squash,
    FastForward,
}

#[derive(Clone, Copy, Debug)]
enum Change {
    Closure,
    NewDocument,
    EditedDocument,
}

impl Change {
    fn edits(self) -> Vec<(String, Option<String>)> {
        match self {
            Self::Closure => vec![put(&ticket_path(TICKET), ticket(TICKET, CLOSED, "body"))],
            Self::NewDocument => vec![put("docs/new.md", document(NEW_DOC, "v1"))],
            Self::EditedDocument => vec![put("docs/old.md", document(OLD_DOC, "v2"))],
        }
    }

    fn arrival(self) -> (AuthoringKind, &'static str) {
        match self {
            Self::Closure => (AuthoringKind::Ticket, TICKET),
            Self::NewDocument => (AuthoringKind::Document, NEW_DOC),
            Self::EditedDocument => (AuthoringKind::Document, OLD_DOC),
        }
    }
}

/// What a row expects of `arrivals`: each arrival's kind, ID, commit and
/// seconds, in the order it lists them, the deleted documents, and how many
/// commits the walk diffed.
struct Expected {
    arrived: Vec<(AuthoringKind, &'static str, Oid, i64)>,
    deleted: Vec<&'static str>,
    diffed: usize,
}

struct Row {
    name: String,
    before: Oid,
    after: Oid,
    expected: Expected,
}

fn row(
    name: impl Into<String>,
    before: Oid,
    after: Oid,
    arrived: Vec<(AuthoringKind, &'static str, Oid, i64)>,
    deleted: Vec<&'static str>,
    diffed: usize,
) -> Row {
    Row {
        name: name.into(),
        before,
        after,
        expected: Expected {
            arrived,
            deleted,
            diffed,
        },
    }
}

fn styled_row(fixture: &Fixture, style: Style, change: Change) -> Row {
    let base = base_files();
    let root = fixture.commit(&base, &[], 100);
    let (kind, id) = change.arrival();
    let name = format!("{style:?} {change:?}");
    match style {
        Style::MergeCommit => {
            // The primary moved on by itself while the branch was open.
            let primary_files = with(&base, &[put("README", "two\n".into())]);
            let primary = fixture.commit(&primary_files, &[root], 300);
            let branch = fixture.commit(&with(&base, &change.edits()), &[root], 200);
            let merged = fixture.commit(
                &with(&primary_files, &change.edits()),
                &[primary, branch],
                400,
            );
            row(
                name,
                primary,
                merged,
                vec![(kind, id, merged, 400)],
                vec![],
                1,
            )
        }
        Style::Squash => {
            let squash = fixture.commit(&with(&base, &change.edits()), &[root], 500);
            row(name, root, squash, vec![(kind, id, squash, 500)], vec![], 1)
        }
        Style::FastForward => {
            // The branch's own commit changes the item; a later one does not.
            let changed = fixture.commit(&with(&base, &change.edits()), &[root], 200);
            let after_files = with(
                &with(&base, &change.edits()),
                &[put("README", "two\n".into())],
            );
            let last = fixture.commit(&after_files, &[changed], 300);
            row(name, root, last, vec![(kind, id, changed, 200)], vec![], 2)
        }
    }
}

fn rows(fixture: &Fixture) -> Vec<Row> {
    let mut rows = Vec::new();
    for style in [Style::MergeCommit, Style::Squash, Style::FastForward] {
        for change in [Change::Closure, Change::NewDocument, Change::EditedDocument] {
            // Each row builds its own history in the one repository.
            rows.push(styled_row(fixture, style, change));
        }
    }
    let base = base_files();
    let root = fixture.commit(&base, &[], 100);

    // A fast-forward whose branch merged the primary in: the arrival is the
    // branch commit that closed the ticket, not the merge, though the merge
    // differs from the primary's commit by the closure.
    let closed = with(
        &base,
        &[put(&ticket_path(TICKET), ticket(TICKET, CLOSED, "body"))],
    );
    let primary = fixture.commit(&with(&base, &[put("README", "two\n".into())]), &[root], 300);
    let branch = fixture.commit(&closed, &[root], 200);
    let merged_in = fixture.commit(
        &with(&closed, &[put("README", "two\n".into())]),
        &[branch, primary],
        350,
    );
    rows.push(row(
        "fast-forward that merged the primary in",
        primary,
        merged_in,
        vec![(AuthoringKind::Ticket, TICKET, branch, 200)],
        vec![],
        2,
    ));

    // Two arrivals at different commits of one range.
    let first = fixture.commit(&closed, &[root], 200);
    let both = with(&closed, &[put("docs/new.md", document(NEW_DOC, "v1"))]);
    let second = fixture.commit(&both, &[first], 300);
    rows.push(row(
        "two arrivals in one range",
        root,
        second,
        vec![
            (AuthoringKind::Document, NEW_DOC, second, 300),
            (AuthoringKind::Ticket, TICKET, first, 200),
        ],
        vec![],
        2,
    ));

    // A ticket saved after its closure, on its branch: the save's time.
    let saved = fixture.commit(
        &with(
            &closed,
            &[put(&ticket_path(TICKET), ticket(TICKET, CLOSED, "edited"))],
        ),
        &[first],
        300,
    );
    rows.push(row(
        "ticket saved after its closure",
        root,
        saved,
        vec![(AuthoringKind::Ticket, TICKET, saved, 300)],
        vec![],
        1,
    ));

    // A ticket file changed with no closed_at is no arrival and costs no walk.
    let edited = fixture.commit(
        &with(
            &base,
            &[put(&ticket_path(TICKET), ticket(TICKET, "", "edited"))],
        ),
        &[root],
        200,
    );
    rows.push(row(
        "ticket changed, still open",
        root,
        edited,
        vec![],
        vec![],
        0,
    ));

    // A ticket whose before already had closed_at: a save is no arrival, and
    // a closed_at that changed is one again.
    let was_closed = fixture.commit(&closed, &[root], 200);
    let saved_closed = fixture.commit(
        &with(
            &closed,
            &[put(&ticket_path(TICKET), ticket(TICKET, CLOSED, "edited"))],
        ),
        &[was_closed],
        300,
    );
    rows.push(row(
        "ticket already closed, saved",
        was_closed,
        saved_closed,
        vec![],
        vec![],
        0,
    ));
    let closed_again = fixture.commit(
        &with(
            &closed,
            &[put(
                &ticket_path(TICKET),
                ticket(TICKET, CLOSED_AGAIN, "body"),
            )],
        ),
        &[was_closed],
        400,
    );
    rows.push(row(
        "ticket closed again",
        was_closed,
        closed_again,
        vec![(AuthoringKind::Ticket, TICKET, closed_again, 400)],
        vec![],
        1,
    ));

    // A document deleted is no arrival, and its row goes.
    let removed = fixture.commit(&with(&base, &[("docs/old.md".into(), None)]), &[root], 200);
    rows.push(row(
        "document deleted",
        root,
        removed,
        vec![],
        vec![OLD_DOC],
        0,
    ));

    // A document moved arrives again at its new path, and is not deleted.
    let moved = fixture.commit(
        &with(
            &base,
            &[
                ("docs/old.md".into(), None),
                put("docs/nested/old.md", document(OLD_DOC, "v1")),
            ],
        ),
        &[root],
        200,
    );
    rows.push(row(
        "document moved",
        root,
        moved,
        vec![(AuthoringKind::Document, OLD_DOC, moved, 200)],
        vec![],
        1,
    ));

    // Before that after does not descend from, and before equal to after.
    rows.push(row(
        "before is not an ancestor",
        primary,
        branch,
        vec![],
        vec![],
        0,
    ));
    rows.push(row("before equals after", root, root, vec![], vec![], 0));

    // A candidate no commit of the walk changed: before holds a commit that
    // closed the ticket, then reopened it, and after merges the closed
    // ticket back with a first parent that never touched the file.
    let closed_base = fixture.commit(&closed, &[root], 150);
    let reopened = fixture.commit(&base, &[closed_base], 300);
    let side = fixture.commit(
        &with(&closed, &[put("README", "side\n".into())]),
        &[closed_base],
        250,
    );
    let healed = fixture.commit(
        &with(&closed, &[put("README", "side\n".into())]),
        &[side, reopened],
        400,
    );
    rows.push(row(
        "unassigned candidate",
        reopened,
        healed,
        vec![],
        vec![],
        2,
    ));
    rows
}

// The arrivals of a range are decided by one function for every move, so one
// table pins the rule: what each kind of integration brings, the time it is
// given, and how few commits the walk reads to give it.
#[test]
fn arrivals_follow_the_range() {
    let fixture = Fixture::new();
    let mut failures = Vec::new();
    for row in rows(&fixture) {
        take_diffed();
        let actual = arrivals(&fixture.repository, row.before, row.after).unwrap();
        let diffed = take_diffed();
        let arrived: Vec<_> = actual
            .arrived
            .iter()
            .map(|arrival| {
                (
                    arrival.kind,
                    arrival.item_id.to_string(),
                    arrival.commit,
                    arrival.merged_at.unix_timestamp(),
                )
            })
            .collect();
        let expected: Vec<_> = row
            .expected
            .arrived
            .iter()
            .map(|(kind, id, commit, seconds)| (*kind, (*id).to_owned(), *commit, *seconds))
            .collect();
        let deleted: Vec<_> = actual.deleted.iter().map(ToString::to_string).collect();
        let expected_deleted: Vec<_> = row
            .expected
            .deleted
            .iter()
            .map(|id| (*id).to_owned())
            .collect();
        if arrived != expected || deleted != expected_deleted || diffed != row.expected.diffed {
            failures.push(format!(
                "{}: arrived {arrived:?} deleted {deleted:?} diffed {diffed}; expected {expected:?} {expected_deleted:?} {}",
                row.name, row.expected.diffed
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn registry() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let mut connection =
        open_registry(&directory.path().join("index.sqlite3"), &mut |_| {}).unwrap();
    migrate_registry(&mut connection).unwrap();
    connection
        .execute(
            "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
             VALUES (1, '/repository', 1, 'accessible', 0)",
            [],
        )
        .unwrap();
    (directory, connection)
}

fn arrival(kind: AuthoringKind, id: &str, commit: Oid, seconds: i64) -> Arrival {
    Arrival {
        kind,
        item_id: id.parse().unwrap(),
        commit,
        merged_at: OffsetDateTime::from_unix_timestamp(seconds).unwrap(),
    }
}

fn record_now(
    connection: &mut Connection,
    repository: &Repository,
    operation: &str,
    arrivals: Arrivals,
) {
    let transaction = connection.transaction().unwrap();
    record(
        &transaction,
        repository,
        1,
        &OperationId::parse(operation).unwrap(),
        &arrivals,
    )
    .unwrap();
    transaction.commit().unwrap();
}

fn rows_of(connection: &Connection) -> Vec<(String, String, String, i64, String)> {
    connection
        .prepare(
            "SELECT kind, item_ulid, commit_oid, merged_at, operation_ulid
               FROM item_merges ORDER BY kind, item_ulid",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

const FIRST_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
const SECOND_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";
const THIRD_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB3";
const FOURTH_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB4";

// Two sites can record one item out of order, so a row is replaced only by an
// arrival at a descendant of its commit; a deleted document's row goes, a
// ticket's of the same ID stays, and a time the table cannot hold costs only
// its own row.
#[test]
fn record_keeps_the_newer_arrival() {
    let fixture = Fixture::new();
    let older = fixture.commit(&base_files(), &[], 100);
    let newer = fixture.commit(&base_files(), &[older], 200);
    let newest = fixture.commit(&base_files(), &[newer], 300);
    let unrelated = fixture.commit(&base_files(), &[], 400);
    let (_directory, mut connection) = registry();
    let at = |commit: Oid| commit.to_string();

    record_now(
        &mut connection,
        &fixture.repository,
        FIRST_OPERATION,
        Arrivals {
            arrived: vec![arrival(AuthoringKind::Ticket, TICKET, newer, 200)],
            deleted: vec![],
        },
    );
    // The older arrival records second, and the row stays.
    record_now(
        &mut connection,
        &fixture.repository,
        SECOND_OPERATION,
        Arrivals {
            arrived: vec![arrival(AuthoringKind::Ticket, TICKET, older, 100)],
            deleted: vec![],
        },
    );
    assert_eq!(
        rows_of(&connection),
        vec![(
            "ticket".into(),
            TICKET.into(),
            at(newer),
            200,
            FIRST_OPERATION.into()
        )]
    );
    // An arrival the row's commit does not precede stays out too.
    record_now(
        &mut connection,
        &fixture.repository,
        SECOND_OPERATION,
        Arrivals {
            arrived: vec![arrival(AuthoringKind::Ticket, TICKET, unrelated, 400)],
            deleted: vec![],
        },
    );
    assert_eq!(rows_of(&connection)[0].2, at(newer));
    // A descendant replaces it, operation and time with it.
    record_now(
        &mut connection,
        &fixture.repository,
        THIRD_OPERATION,
        Arrivals {
            arrived: vec![arrival(AuthoringKind::Ticket, TICKET, newest, 300)],
            deleted: vec![],
        },
    );
    assert_eq!(
        rows_of(&connection),
        vec![(
            "ticket".into(),
            TICKET.into(),
            at(newest),
            300,
            THIRD_OPERATION.into()
        )]
    );

    // A document's row goes when the document is deleted; the ticket's row of
    // the same ID, and the other document's, stay. A negative time is skipped
    // and the arrival after it is recorded.
    record_now(
        &mut connection,
        &fixture.repository,
        FOURTH_OPERATION,
        Arrivals {
            arrived: vec![
                arrival(AuthoringKind::Document, OLD_DOC, newer, 200),
                arrival(AuthoringKind::Document, NEW_DOC, newer, -1),
                arrival(AuthoringKind::Document, TICKET, newer, 200),
            ],
            deleted: vec![],
        },
    );
    assert_eq!(rows_of(&connection).len(), 3);
    record_now(
        &mut connection,
        &fixture.repository,
        FOURTH_OPERATION,
        Arrivals {
            arrived: vec![],
            deleted: vec![TICKET.parse().unwrap(), NEW_DOC.parse().unwrap()],
        },
    );
    assert_eq!(
        rows_of(&connection)
            .into_iter()
            .map(|(kind, id, ..)| (kind, id))
            .collect::<Vec<_>>(),
        vec![
            ("document".to_owned(), OLD_DOC.to_owned()),
            ("ticket".to_owned(), TICKET.to_owned())
        ]
    );
}
