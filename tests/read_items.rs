//! Document and ticket lists, complete item reads, and what the index
//! stores for them.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use manyhands::repository::{
    EnableRepositoryOutcome, FailurePoint, OperationId, RebuildRepositoryRequest, RefreshOutcome,
    RefreshRepositoryRequest, RepositoryErrorKind, RepositoryService,
};
use rusqlite::Connection;

mod support;

const TICKET_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const TICKET_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

fn refresh(service: &RepositoryService, root: &Path) -> RefreshOutcome {
    refresh_as(service, root, support::operation_id())
}

/// A refresh that did not complete is retried under its own operation ID.
fn refresh_as(
    service: &RepositoryService,
    root: &Path,
    operation_id: OperationId,
) -> RefreshOutcome {
    service
        .refresh_repository(RefreshRepositoryRequest {
            root: root.to_owned(),
            operation_id,
        })
        .unwrap()
}

fn refresh_completely(service: &RepositoryService, root: &Path) {
    assert!(matches!(
        refresh(service, root),
        RefreshOutcome::Refreshed { .. }
    ));
}

fn rebuild(service: &RepositoryService, root: &Path) {
    rebuild_as(service, root, support::operation_id());
}

fn rebuild_as(service: &RepositoryService, root: &Path, operation_id: OperationId) {
    service
        .rebuild_repository(RebuildRepositoryRequest {
            root: root.to_owned(),
            operation_id,
        })
        .unwrap();
}

fn index(data_directory: &Path) -> Connection {
    Connection::open(data_directory.join("manyhands.sqlite3")).unwrap()
}

fn write(root: &Path, path: &str, source: &str) -> PathBuf {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    path
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// A ticket's source. `extra` is front matter lines, each ending in a
/// newline, placed after the required fields.
fn ticket_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n{extra}---\nBody of {title}.\n"
    )
}

fn now() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}

fn refreshed_at(data_directory: &Path) -> Vec<Option<i64>> {
    index(data_directory)
        .prepare("SELECT refreshed_at FROM repositories ORDER BY root_path")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn refresh_required(data_directory: &Path) -> Vec<bool> {
    index(data_directory)
        .prepare("SELECT refresh_required FROM repositories ORDER BY root_path")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// `(item ID, closed_by, unknown_metadata)` for every stored item.
fn stored_item_columns(data_directory: &Path) -> Vec<(String, Option<String>, Option<String>)> {
    index(data_directory)
        .prepare(
            "SELECT item_id, closed_by, unknown_metadata FROM discovered_items ORDER BY item_id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn columns(connection: &Connection, table: &str) -> Vec<String> {
    connection
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn write_closed_and_open_tickets(root: &Path) {
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(
            TICKET_A,
            "Closed",
            "closed_at: 2026-09-30T12:34:56Z\nclosed_by: Ada Lovelace <ada@example.invalid>\n\
             zeta: last\nalpha:\n  nested: [1, two, null]\n",
        ),
    );
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "Open", ""),
    );
}

fn assert_closed_and_open_tickets_are_stored(data_directory: &Path) {
    let stored = stored_item_columns(data_directory);
    assert_eq!(stored.len(), 2, "{stored:?}");
    assert_eq!(stored[0].0, TICKET_A);
    assert_eq!(
        stored[0].1.as_deref(),
        Some("Ada Lovelace <ada@example.invalid>")
    );
    // One JSON object, its keys in order at every depth.
    assert_eq!(
        stored[0].2.as_deref(),
        Some(
            r#"{"not_representable":false,"values":{"alpha":{"nested":[1,"two",null]},"zeta":"last"}}"#
        )
    );
    assert_eq!(stored[1].0, TICKET_B);
    assert_eq!(stored[1].1, None);
    assert_eq!(
        stored[1].2.as_deref(),
        Some(r#"{"not_representable":false,"values":{}}"#)
    );
}

#[test]
fn refresh_stores_closed_by_and_unknown_metadata_with_each_item() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    write_closed_and_open_tickets(&fixture.root);

    refresh_completely(&enabled.service, &fixture.root);

    assert_closed_and_open_tickets_are_stored(enabled.data_directory.path());
    assert_git_transport_uninitialized();
}

#[test]
fn rebuild_stores_closed_by_and_unknown_metadata_with_each_item() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    write_closed_and_open_tickets(&fixture.root);

    rebuild(&enabled.service, &fixture.root);

    assert_closed_and_open_tickets_are_stored(enabled.data_directory.path());
    assert_git_transport_uninitialized();
}

#[test]
fn a_metadata_value_with_no_json_form_is_stored_as_null_and_flagged() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    write(
        &fixture.root,
        &ticket_path(TICKET_A),
        &ticket_source(
            TICKET_A,
            "Odd",
            "tagged: !secret hidden\nnot_a_number: .nan\nendless: .inf\n\
             keyed:\n  1: one\nfine: 1.5\n? [a, b]\n: complex\n",
        ),
    );

    refresh_completely(&enabled.service, &fixture.root);

    let stored = stored_item_columns(enabled.data_directory.path());
    assert_eq!(
        stored[0].2.as_deref(),
        Some(
            r#"{"not_representable":true,"values":{"endless":null,"fine":1.5,"keyed":null,"not_a_number":null,"tagged":null}}"#
        )
    );
    assert_git_transport_uninitialized();
}

/// A registration whose first refresh, the one enabling starts, did not
/// complete. `operation_id` is the pending operation a retry resumes.
struct NeverRefreshed {
    fixture: support::TestRepository,
    data: tempfile::TempDir,
    service: RepositoryService,
    operation_id: OperationId,
}

fn never_refreshed_repository() -> NeverRefreshed {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let operation_id = support::operation_id();
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let outcome = failing
        .enable(support::enable_request_with_operation_id(
            &fixture.root,
            operation_id,
        ))
        .unwrap();
    assert!(
        matches!(outcome, EnableRepositoryOutcome::IndexPending(_)),
        "{outcome:?}"
    );
    drop(failing);
    let service = RepositoryService::open_at(data.path()).unwrap();
    NeverRefreshed {
        fixture,
        data,
        service,
        operation_id,
    }
}

#[test]
fn refreshed_at_is_set_only_when_a_refresh_completes() {
    let never = never_refreshed_repository();
    let (root, data) = (&never.fixture.root, never.data.path());
    let document = support::write_document_source(root, "docs/visible.md");
    // Registered, and no refresh has completed.
    assert_eq!(refreshed_at(data), [None]);
    assert_eq!(refresh_required(data), [true]);

    // A refresh that finds the repository changing under it.
    never.service.set_observation_hook_for_testing({
        let document = document.clone();
        move || fs::write(document, "ordinary markdown\n").unwrap()
    });
    assert!(matches!(
        refresh_as(&never.service, root, never.operation_id),
        RefreshOutcome::RetryRequired { .. }
    ));
    assert_eq!(refreshed_at(data), [None]);
    assert_eq!(refresh_required(data), [true]);

    let before = now();
    assert!(matches!(
        refresh_as(&never.service, root, never.operation_id),
        RefreshOutcome::Refreshed { .. }
    ));
    let after = now();

    let [Some(stored)] = refreshed_at(data)[..] else {
        panic!("a completed refresh records its time");
    };
    assert!(
        (before..=after).contains(&stored),
        "{before} {stored} {after}"
    );
    assert_eq!(refresh_required(data), [false]);

    // A later refresh that fails, and one that must be retried, leave the
    // time of the last completed one alone.
    index(data)
        .execute("UPDATE repositories SET refreshed_at = 17", [])
        .unwrap();
    let operation_id = support::operation_id();
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data);
    assert_eq!(
        failing
            .refresh_repository(RefreshRepositoryRequest {
                root: root.clone(),
                operation_id,
            })
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert_eq!(refreshed_at(data), [Some(17)]);
    never.service.set_observation_hook_for_testing({
        let document = document.clone();
        move || fs::write(document, "changed again\n").unwrap()
    });
    assert!(matches!(
        refresh_as(&never.service, root, operation_id),
        RefreshOutcome::RetryRequired { .. }
    ));
    assert_eq!(refreshed_at(data), [Some(17)]);
    assert_eq!(refresh_required(data), [true]);
    assert_git_transport_uninitialized();
}

#[test]
fn enabling_a_repository_completes_its_first_refresh() {
    let fixture = support::born_repository();
    let before = now();
    let enabled = support::enabled_repository(&fixture);
    let after = now();

    let [Some(stored)] = refreshed_at(enabled.data_directory.path())[..] else {
        panic!("enabling refreshes the index");
    };
    assert!(
        (before..=after).contains(&stored),
        "{before} {stored} {after}"
    );
    assert_git_transport_uninitialized();
}

#[test]
fn refreshed_at_is_set_only_when_a_rebuild_persists_its_observation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let data = enabled.data_directory.path();
    let document = support::write_document_source(&fixture.root, "docs/visible.md");
    index(data)
        .execute("UPDATE repositories SET refreshed_at = 17", [])
        .unwrap();

    // A rebuild that finds the repository changing under it.
    enabled.service.set_observation_hook_for_testing({
        let document = document.clone();
        move || fs::write(document, "ordinary markdown\n").unwrap()
    });
    let operation_id = support::operation_id();
    rebuild_as(&enabled.service, &fixture.root, operation_id);
    assert_eq!(refreshed_at(data), [Some(17)]);
    assert_eq!(refresh_required(data), [true]);

    let before = now();
    rebuild_as(&enabled.service, &fixture.root, operation_id);
    let after = now();

    let [Some(stored)] = refreshed_at(data)[..] else {
        panic!("a completed rebuild records its time");
    };
    assert!(
        (before..=after).contains(&stored),
        "{before} {stored} {after}"
    );
    assert_eq!(refresh_required(data), [false]);

    // A rebuild into an index that holds nothing records it as well.
    let empty = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(empty.path()).unwrap();
    rebuild(&service, &fixture.root);
    assert!(matches!(refreshed_at(empty.path())[..], [Some(stored)] if stored >= before));
    assert_git_transport_uninitialized();
}

const ADDED_COLUMNS: [(&str, &str); 3] = [
    ("discovered_items", "closed_by"),
    ("discovered_items", "unknown_metadata"),
    ("repositories", "refreshed_at"),
];

/// Makes the index what a build from before these columns left: the same
/// tables and rows without them.
fn remove_added_columns(data_directory: &Path) {
    let connection = index(data_directory);
    for (table, column) in ADDED_COLUMNS {
        connection
            .execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column}"))
            .unwrap();
        assert!(!columns(&connection, table).contains(&column.to_owned()));
    }
}

fn table_definitions(data_directory: &Path) -> Vec<String> {
    index(data_directory)
        .prepare("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn an_index_from_before_the_item_columns_gains_them_and_is_marked_for_refresh() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    {
        let service = RepositoryService::open_at(data.path()).unwrap();
        for fixture in [&first, &second] {
            service
                .enable(support::enable_request(&fixture.root))
                .unwrap();
            write_closed_and_open_tickets(&fixture.root);
            refresh_completely(&service, &fixture.root);
        }
    }
    assert_eq!(refresh_required(data.path()), [false, false]);
    remove_added_columns(data.path());

    RepositoryService::open_at(data.path()).unwrap();

    let connection = index(data.path());
    for (table, column) in ADDED_COLUMNS {
        assert!(
            columns(&connection, table).contains(&column.to_owned()),
            "{table}.{column}"
        );
    }
    drop(connection);
    // Every registration, not only the first, and no time is invented.
    assert_eq!(refresh_required(data.path()), [true, true]);
    assert_eq!(refreshed_at(data.path()), [None, None]);
    // The rows are kept; what was never observed is absent, not guessed.
    let stored = stored_item_columns(data.path());
    assert_eq!(stored.len(), 4);
    assert!(
        stored
            .iter()
            .all(|(_, closed_by, unknown)| closed_by.is_none() && unknown.is_none())
    );

    // Migrating again changes nothing: the schema is the same, and a
    // registration refreshed since is not marked a second time.
    let definitions = table_definitions(data.path());
    index(data.path())
        .execute("UPDATE repositories SET refresh_required = 0", [])
        .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(table_definitions(data.path()), definitions);
    assert_eq!(refresh_required(data.path()), [false, false]);
    assert_git_transport_uninitialized();
}

#[test]
fn a_new_index_has_the_item_columns_and_marks_nothing() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let data = enabled.data_directory.path();

    let connection = index(data);
    for (table, column) in ADDED_COLUMNS {
        assert!(
            columns(&connection, table).contains(&column.to_owned()),
            "{table}.{column}"
        );
    }
    refresh_completely(&enabled.service, &fixture.root);
    // Opening the same index again is not a migration that adds anything.
    RepositoryService::open_at(data).unwrap();
    assert_eq!(refresh_required(data), [false]);
    assert_git_transport_uninitialized();
}
