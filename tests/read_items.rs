//! Document and ticket lists, complete item reads, and what the index
//! stores for them.

// A read error carries its whole scope by value, as the contract has it.
#![allow(clippy::result_large_err)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use manyhands::{
    repository::{
        ChangeSource, ClosureFilter, ClosureState, FailurePoint, IndexState, ItemContextKind,
        ItemDto, ItemDtoKind, ItemListDto, ReadError, ReadinessFilter, ReadinessReasonCode,
        ReadinessState, RefreshOutcome, RefreshRepositoryRequest, RepositoryErrorKind,
        RepositoryService, TicketFilter,
    },
    results::{Envelope, Outcome, ProblemCode, ResultCode},
};
use rusqlite::Connection;
use serde_json::{Value, json};
use support::items::{
    CLOSURE, DOCUMENT_A, DOCUMENT_B, DOCUMENT_C, TICKET_A, TICKET_B, TICKET_C, commit,
    context_worktree, create_document_context, degraded_service, document_source, index, item_id,
    never_refreshed_repository, rebuild, rebuild_as, refresh, refresh_as, refresh_completely,
    ticket_path, ticket_source, ticket_source_with, write,
};

mod support;

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
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

// ---------------------------------------------------------------------
// Lists and complete reads.
// ---------------------------------------------------------------------

const DOCUMENT_D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
const DOCUMENT_E: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
const TICKET_D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";

/// Planted in YAML that fails to parse, where the parser's message
/// repeats it.
const SENTINEL: &str = "SENTINEL-5e17";

fn enabled() -> (support::TestRepository, support::EnabledRepository) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    (fixture, enabled)
}

fn root_string(fixture: &support::TestRepository) -> String {
    fs::canonicalize(&fixture.root)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

fn ids(list: &ItemListDto) -> Vec<Option<&str>> {
    list.items.iter().map(|item| item.id.as_deref()).collect()
}

fn paths(list: &ItemListDto) -> Vec<&str> {
    list.items.iter().map(|item| item.path.as_str()).collect()
}

fn codes(item: &ItemDto) -> Vec<ProblemCode> {
    item.problems.iter().map(|problem| problem.code).collect()
}

fn recovery(error: &ReadError) -> Value {
    serde_json::to_value(&error.to_envelope::<Value>("item show").recovery).unwrap()
}

fn refresh_recovery(root: &str) -> Value {
    json!([{"action": "index.refresh", "operation_id": null, "arguments": {"root": root}}])
}

fn all_tickets(
    service: &RepositoryService,
    repo: &manyhands::repository::ResolvedRepository,
) -> ItemListDto {
    service
        .list_tickets(repo, &TicketFilter::default())
        .unwrap()
}

/// What every list entry leaves out, and what an item whose file has no
/// relationship fields has.
fn assert_list_form(item: &ItemDto) {
    assert_eq!(item.body, None, "{item:?}");
    assert_eq!(item.source, None, "{item:?}");
    assert_eq!(item.observation, None, "{item:?}");
    assert_eq!(item.slug, None);
    assert_eq!(item.parent, None);
    assert!(item.deps.is_empty());
    // A ticket that depends on nothing has no reason to wait. A document,
    // and a file that is not an item, has no readiness at all.
    match (&item.id, item.kind) {
        (Some(_), ItemDtoKind::Ticket) => {
            let readiness = item.readiness.as_ref().unwrap();
            assert_ne!(readiness.state, ReadinessState::Blocked);
            assert!(readiness.reasons.is_empty());
        }
        _ => assert_eq!(item.readiness, None),
    }
}

#[test]
fn documents_are_listed_by_path_then_id() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    // Two documents at one path: the item worktree's, and the primary's.
    // The index stores the primary's first.
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/same.md");
    write(
        root,
        "docs/same.md",
        &document_source(DOCUMENT_C, "Same", ""),
    );
    write(root, "docs/b.md", &document_source(DOCUMENT_B, "B", ""));
    // As paths, `a/z.md` sorts before `a.md`; as text it sorts after.
    write(root, "docs/a/z.md", &document_source(DOCUMENT_D, "Z", ""));
    write(root, "docs/a.md", &document_source(DOCUMENT_E, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = enabled.service.list_documents(&repo).unwrap();

    assert!(list.complete);
    assert_eq!(list.index.state, IndexState::Current);
    assert_eq!(
        paths(&list),
        [
            "docs/a.md",
            "docs/a/z.md",
            "docs/b.md",
            "docs/same.md",
            "docs/same.md"
        ]
    );
    assert_eq!(
        ids(&list),
        [
            Some(DOCUMENT_E),
            Some(DOCUMENT_D),
            Some(DOCUMENT_B),
            Some(DOCUMENT_A),
            Some(DOCUMENT_C)
        ]
    );
    for item in &list.items {
        assert_list_form(item);
        assert_eq!(item.kind, ItemDtoKind::Document);
        assert_eq!(item.ticket_type, None);
        assert_eq!(item.status, None);
        assert_eq!(item.closure, None);
        assert!(item.title.is_some());
        assert!(item.changed_at.is_some());
        assert!(item.change_source.is_some());
        assert_eq!(item.index, list.index);
        assert!(item.problems.is_empty());
    }
    // No ticket list holds a document, and no document list a ticket.
    assert!(all_tickets(&enabled.service, &repo).items.is_empty());
    assert_git_transport_uninitialized();
}

#[test]
fn tickets_are_listed_by_change_time_latest_first_then_id() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    for (id, seconds) in [(TICKET_A, 1_000), (TICKET_C, 3_000), (TICKET_B, 3_000)] {
        write(root, &ticket_path(id), &ticket_source(id, id, ""));
        commit(&fixture, &[&ticket_path(id)], seconds);
    }
    // Not committed: changed when the file was written, which is now.
    write(
        root,
        &ticket_path(TICKET_D),
        &ticket_source(TICKET_D, "D", ""),
    );
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = all_tickets(&enabled.service, &repo);

    // The index stores them in path order, which is A, B, C, D.
    assert_eq!(
        ids(&list),
        [
            Some(TICKET_D),
            Some(TICKET_B),
            Some(TICKET_C),
            Some(TICKET_A)
        ]
    );
    assert_eq!(
        list.items
            .iter()
            .map(|item| (item.changed_at.as_deref(), item.change_source))
            .skip(1)
            .collect::<Vec<_>>(),
        [
            (Some("1970-01-01T00:50:00Z"), Some(ChangeSource::GitCommit)),
            (Some("1970-01-01T00:50:00Z"), Some(ChangeSource::GitCommit)),
            (Some("1970-01-01T00:16:40Z"), Some(ChangeSource::GitCommit)),
        ]
    );
    assert_eq!(list.items[0].change_source, Some(ChangeSource::Uncommitted));
    for item in &list.items {
        assert_list_form(item);
        assert_eq!(item.kind, ItemDtoKind::Ticket);
        assert_eq!(item.ticket_type.as_deref(), Some("task"));
        assert_eq!(item.status.as_deref(), Some("open"));
        assert_eq!(item.closure.as_ref().unwrap().state, ClosureState::Open);
    }
    assert_eq!(
        paths(&enabled.service.list_documents(&repo).unwrap()),
        ["docs/a.md"]
    );
    assert_git_transport_uninitialized();
}

#[test]
fn ticket_filters_match_exactly_and_closure_follows_lifecycle_metadata() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    // Open, in project `alpha`.
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source_with(TICKET_A, "A", "task", "open", "project: alpha\n"),
    );
    // Its status text says closed; nothing closed it.
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source_with(TICKET_B, "B", "bug", "closed", "project: Alpha\n"),
    );
    // Closed by the lifecycle, whatever its status text says.
    write(
        root,
        &ticket_path(TICKET_C),
        &ticket_source_with(TICKET_C, "C", "task", "open", CLOSURE),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let listed = |filter: TicketFilter| {
        let mut ids: Vec<String> = enabled
            .service
            .list_tickets(&repo, &filter)
            .unwrap()
            .items
            .into_iter()
            .map(|item| item.id.unwrap())
            .collect();
        ids.sort();
        ids
    };
    let text = |value: &str| Some(value.to_owned());

    assert_eq!(TicketFilter::default().closure, ClosureFilter::All);
    assert_eq!(
        listed(TicketFilter::default()),
        [TICKET_A, TICKET_B, TICKET_C]
    );
    for (filter, expected) in [
        (
            TicketFilter {
                status: text("open"),
                ..Default::default()
            },
            vec![TICKET_A, TICKET_C],
        ),
        (
            TicketFilter {
                status: text("closed"),
                ..Default::default()
            },
            vec![TICKET_B],
        ),
        (
            TicketFilter {
                status: text("Open"),
                ..Default::default()
            },
            vec![],
        ),
        (
            TicketFilter {
                status: text("ope"),
                ..Default::default()
            },
            vec![],
        ),
        (
            TicketFilter {
                ticket_type: text("task"),
                ..Default::default()
            },
            vec![TICKET_A, TICKET_C],
        ),
        (
            TicketFilter {
                ticket_type: text("bug"),
                ..Default::default()
            },
            vec![TICKET_B],
        ),
        (
            TicketFilter {
                ticket_type: text("Bug"),
                ..Default::default()
            },
            vec![],
        ),
        (
            TicketFilter {
                project: text("alpha"),
                ..Default::default()
            },
            vec![TICKET_A],
        ),
        (
            TicketFilter {
                project: text("Alpha"),
                ..Default::default()
            },
            vec![TICKET_B],
        ),
        (
            TicketFilter {
                project: text("beta"),
                ..Default::default()
            },
            vec![],
        ),
        // A ticket whose status text is `closed` is open; the one the
        // lifecycle closed is not, though its status text is `open`.
        (
            TicketFilter {
                closure: ClosureFilter::Open,
                ..Default::default()
            },
            vec![TICKET_A, TICKET_B],
        ),
        (
            TicketFilter {
                closure: ClosureFilter::Closed,
                ..Default::default()
            },
            vec![TICKET_C],
        ),
        (
            TicketFilter {
                closure: ClosureFilter::All,
                ..Default::default()
            },
            vec![TICKET_A, TICKET_B, TICKET_C],
        ),
        // Filters combine.
        (
            TicketFilter {
                status: text("open"),
                closure: ClosureFilter::Closed,
                ..Default::default()
            },
            vec![TICKET_C],
        ),
        (
            TicketFilter {
                ticket_type: text("task"),
                project: text("alpha"),
                closure: ClosureFilter::Open,
                ..Default::default()
            },
            vec![TICKET_A],
        ),
        (
            TicketFilter {
                status: text("closed"),
                closure: ClosureFilter::Closed,
                ..Default::default()
            },
            vec![],
        ),
        // None of them depends on anything: the open ones are ready, the
        // one whose status says closed among them, and the closed one is
        // neither ready nor blocked.
        (
            TicketFilter {
                readiness: Some(ReadinessFilter::Ready),
                ..Default::default()
            },
            vec![TICKET_A, TICKET_B],
        ),
        (
            TicketFilter {
                readiness: Some(ReadinessFilter::Blocked),
                ..Default::default()
            },
            vec![],
        ),
        // None of these tickets has a short code.
        (
            TicketFilter {
                slug: text("mh-vh-k9x2b"),
                ..Default::default()
            },
            vec![],
        ),
    ] {
        assert_eq!(listed(filter.clone()), expected, "{filter:?}");
    }

    let list = all_tickets(&enabled.service, &repo);
    let closure = |id: &str| {
        list.items
            .iter()
            .find(|item| item.id.as_deref() == Some(id))
            .unwrap()
            .closure
            .clone()
            .unwrap()
    };
    let (open, closed) = (closure(TICKET_B), closure(TICKET_C));
    assert_eq!(open.state, ClosureState::Open);
    assert_eq!((open.closed_at, open.closed_by), (None, None));
    assert_eq!(closed.state, ClosureState::Closed);
    assert_eq!(closed.closed_at.as_deref(), Some("2026-09-30T12:34:56Z"));
    assert_eq!(
        closed.closed_by.as_deref(),
        Some("Ada Lovelace <ada@example.invalid>")
    );
    assert_git_transport_uninitialized();
}

#[test]
fn an_item_in_two_other_item_worktrees_is_listed_once_from_primary() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(
        root,
        "docs/shared.md",
        &document_source(DOCUMENT_C, "Shared", ""),
    );
    commit(&fixture, &["docs/shared.md"], 1_000);
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/first.md");
    create_document_context(&enabled.service, root, DOCUMENT_B, "docs/second.md");
    // Both item worktrees hold the shared document as part of their checkout.
    for id in [DOCUMENT_A, DOCUMENT_B] {
        assert!(context_worktree(root, id).join("docs/shared.md").is_file());
    }
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = enabled.service.list_documents(&repo).unwrap();

    assert_eq!(
        ids(&list),
        [Some(DOCUMENT_A), Some(DOCUMENT_B), Some(DOCUMENT_C)]
    );
    for (item, id) in list.items.iter().zip([DOCUMENT_A, DOCUMENT_B]) {
        assert_eq!(item.context.kind, ItemContextKind::Active);
        assert_eq!(
            item.context.branch,
            Some(format!("manyhands/document/{id}"))
        );
        assert_eq!(
            Path::new(&item.context.worktree),
            context_worktree(root, id)
        );
        assert!(item.context.head_oid.is_some());
    }
    let shared = &list.items[2];
    assert_eq!(shared.path, "docs/shared.md");
    assert_eq!(shared.context.kind, ItemContextKind::Primary);
    assert_eq!(shared.context.branch.as_deref(), Some("main"));
    assert_eq!(shared.context.worktree, root_string(&fixture));
    // The complete read agrees with the list about each copy.
    for item in &list.items {
        let shown = enabled
            .service
            .show_item(&repo, &item_id(item.id.as_deref().unwrap()))
            .unwrap();
        assert_eq!(shown.context, item.context);
        assert_eq!(
            shown.source.unwrap(),
            fs::read_to_string(Path::new(&item.context.worktree).join(&item.path)).unwrap()
        );
    }
    assert_git_transport_uninitialized();
}

/// Two tickets, one closed and with unknown metadata, and a document.
fn write_mixed_items(root: &Path) -> Vec<PathBuf> {
    vec![
        write(
            root,
            &ticket_path(TICKET_A),
            &ticket_source(
                TICKET_A,
                "Closed",
                &format!("{CLOSURE}priority: 2\nlabels: [one, two]\n"),
            ),
        ),
        write(
            root,
            &ticket_path(TICKET_B),
            &ticket_source(TICKET_B, "Open", ""),
        ),
        write(
            root,
            "docs/a.md",
            &document_source(DOCUMENT_A, "A", "audience:\n  internal: true\n"),
        ),
    ]
}

#[test]
fn lists_carry_closed_by_and_unknown_metadata_and_open_no_item_file() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let files = write_mixed_items(root);
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let lists = || {
        (
            enabled.service.list_documents(&repo).unwrap(),
            all_tickets(&enabled.service, &repo),
            enabled
                .service
                .list_tickets(
                    &repo,
                    &TicketFilter {
                        closure: ClosureFilter::Closed,
                        ..Default::default()
                    },
                )
                .unwrap(),
        )
    };

    #[cfg(target_os = "linux")]
    let watch = support::open_watch::OpenWatch::on(&files);
    let before = lists();
    #[cfg(target_os = "linux")]
    {
        assert!(
            !watch.saw_an_open_or_a_read(),
            "a list opened or read an item file"
        );
        // The watch does see an open: a complete read is one.
        enabled
            .service
            .show_item(&repo, &item_id(TICKET_A))
            .unwrap();
        assert!(watch.saw_an_open_or_a_read());
    }

    let (documents, tickets, closed) = &before;
    assert_eq!(
        Value::Object(documents.items[0].unknown_metadata.clone()),
        json!({"audience": {"internal": true}})
    );
    let ticket = tickets
        .items
        .iter()
        .find(|item| item.id.as_deref() == Some(TICKET_A))
        .unwrap();
    assert_eq!(
        ticket.closure.as_ref().unwrap().closed_by.as_deref(),
        Some("Ada Lovelace <ada@example.invalid>")
    );
    assert_eq!(
        Value::Object(ticket.unknown_metadata.clone()),
        json!({"labels": ["one", "two"], "priority": 2})
    );
    assert_eq!(ids(closed), [Some(TICKET_A)]);

    // With nothing left to read, the lists are what they were.
    for file in &files {
        fs::remove_file(file).unwrap();
    }
    fs::remove_dir_all(root.join(".manyhands/tickets")).unwrap();
    fs::remove_dir_all(root.join("docs")).unwrap();
    assert!(lists() == before);
    assert_git_transport_uninitialized();
}

#[test]
fn a_migrated_index_lists_what_it_has_and_says_it_is_stale() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write_mixed_items(root);
    refresh_completely(&enabled.service, root);
    let support::EnabledRepository {
        service,
        data_directory,
    } = enabled;
    drop(service);
    remove_added_columns(data_directory.path());

    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    let repo = service.resolve_repository(root).unwrap();
    let tickets = all_tickets(&service, &repo);
    let documents = service.list_documents(&repo).unwrap();

    // The rows are still there, without what the old index never stored.
    assert_eq!(ids(&tickets).len(), 2);
    assert_eq!(ids(&documents), [Some(DOCUMENT_A)]);
    for list in [&tickets, &documents] {
        assert_eq!(list.index.state, IndexState::Stale);
        assert_eq!(list.index.refreshed_at, None);
        for item in &list.items {
            assert!(item.unknown_metadata.is_empty());
            assert_eq!(item.index.state, IndexState::Stale);
        }
    }
    let closed = tickets
        .items
        .iter()
        .find(|item| item.id.as_deref() == Some(TICKET_A))
        .unwrap()
        .closure
        .clone()
        .unwrap();
    assert_eq!(closed.state, ClosureState::Closed);
    assert_eq!(closed.closed_by, None);
    assert_eq!(
        service.list_repositories().unwrap().items[0].index.state,
        IndexState::Stale
    );
    // A complete read uses the file, so it has what the index lacks.
    let shown = service.show_item(&repo, &item_id(TICKET_A)).unwrap();
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_eq!(
        shown.closure.unwrap().closed_by.as_deref(),
        Some("Ada Lovelace <ada@example.invalid>")
    );

    // It holds contexts, so it is stale and not never refreshed, and it has
    // no refresh time to give. A refresh that completes makes it current,
    // and a later one that must be retried makes it stale again.
    refresh_completely(&service, root);
    let tickets = all_tickets(&service, &repo);
    assert_eq!(tickets.index.state, IndexState::Current);
    assert!(tickets.index.refreshed_at.is_some());
    assert!(
        tickets
            .items
            .iter()
            .any(|item| !item.unknown_metadata.is_empty())
    );
    let document = root.join("docs/a.md");
    service.set_observation_hook_for_testing(move || fs::write(document, "plain\n").unwrap());
    assert!(matches!(
        refresh(&service, root),
        RefreshOutcome::RetryRequired { .. }
    ));
    let stale = service.list_documents(&repo).unwrap();
    assert_eq!(stale.index.state, IndexState::Stale);
    assert_eq!(stale.index.refreshed_at, tickets.index.refreshed_at);
    assert_eq!(ids(&stale), [Some(DOCUMENT_A)]);
    assert_git_transport_uninitialized();
}

fn set_modified_later(path: &Path) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() + Duration::from_secs(30))
        .unwrap();
}

#[test]
fn an_item_with_an_active_worktree_is_read_from_it_and_an_unrefreshed_edit_is_stale() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/active.md");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let worktree = context_worktree(root, DOCUMENT_A);
    let file = worktree.join("docs/active.md");
    let stored = fs::read_to_string(&file).unwrap();
    // The primary has no copy: the item exists only in its worktree.
    assert!(!root.join("docs/active.md").exists());

    let shown = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();

    assert_eq!(shown.id.as_deref(), Some(DOCUMENT_A));
    assert_eq!(shown.kind, ItemDtoKind::Document);
    assert_eq!(shown.path, "docs/active.md");
    assert_eq!(shown.title.as_deref(), Some("Draft of docs/active.md"));
    assert_eq!(shown.context.kind, ItemContextKind::Active);
    assert_eq!(Path::new(&shown.context.worktree), worktree);
    assert_eq!(
        shown.context.branch,
        Some(format!("manyhands/document/{DOCUMENT_A}"))
    );
    assert_eq!(shown.source.as_deref(), Some(stored.as_str()));
    assert_eq!(shown.body.as_deref(), Some(""));
    assert!(shown.observation.as_deref().unwrap().starts_with("v1:"));
    assert!(shown.changed_at.is_some());
    assert_eq!(shown.index.state, IndexState::Current);
    assert!(shown.problems.is_empty());

    // Edited in the worktree, and not refreshed.
    let edited = document_source(DOCUMENT_A, "Edited title", "");
    fs::write(&file, &edited).unwrap();
    let after_edit = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();

    assert_eq!(after_edit.source.as_deref(), Some(edited.as_str()));
    assert_eq!(after_edit.title.as_deref(), Some("Edited title"));
    assert_eq!(after_edit.body.as_deref(), Some("# Edited title\n"));
    assert_eq!(after_edit.index.state, IndexState::Stale);
    assert_eq!(after_edit.index.refreshed_at, shown.index.refreshed_at);
    assert_ne!(after_edit.observation, shown.observation);
    assert_eq!(after_edit.context, shown.context);
    // The list still has what the index stored, and says the index is current:
    // it has not looked at the file.
    let list = enabled.service.list_documents(&repo).unwrap();
    assert_eq!(
        list.items[0].title.as_deref(),
        Some("Draft of docs/active.md")
    );
    assert_eq!(list.index.state, IndexState::Current);

    refresh_completely(&enabled.service, root);
    let refreshed = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();
    assert_eq!(refreshed.index.state, IndexState::Current);
    assert_eq!(refreshed.source, after_edit.source);
    assert_eq!(refreshed.observation, after_edit.observation);
    assert_eq!(refreshed.change_source, Some(ChangeSource::Uncommitted));
    assert_git_transport_uninitialized();
}

#[test]
fn an_edit_that_changes_only_the_body_is_stale_once_the_file_is_newer_than_the_refresh() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let show = || {
        enabled
            .service
            .show_item(&repo, &item_id(DOCUMENT_A))
            .unwrap()
    };
    assert_eq!(show().index.state, IndexState::Current);

    let edited = format!("{}More.\n", document_source(DOCUMENT_A, "A", ""));
    fs::write(&file, &edited).unwrap();
    set_modified_later(&file);

    let shown = show();
    assert_eq!(shown.body.as_deref(), Some("# A\nMore.\n"));
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_eq!(
        enabled
            .service
            .show_path(&repo, None, Path::new("docs/a.md"))
            .unwrap()
            .index
            .state,
        IndexState::Stale
    );
    assert_git_transport_uninitialized();
}

/// A valid document and ticket, and three files that are not items: a
/// marker-only document, a ticket whose front matter does not parse, and a
/// ticket filed under `docs/`.
struct Nonconforming {
    fixture: support::TestRepository,
    enabled: support::EnabledRepository,
    marker: String,
    malformed: String,
    misplaced: String,
}

const MARKER_PATH: &str = "docs/marker.md";
const MISPLACED_PATH: &str = "docs/misplaced.md";

fn nonconforming_repository() -> Nonconforming {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source_with(TICKET_A, "A", "task", "open", "project: alpha\n"),
    );
    let marker = "---\nmanyhands_managed: true\n---\nOnly a marker.\n".to_owned();
    write(root, MARKER_PATH, &marker);
    // A key given twice: the parser's message names the key.
    let malformed = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\n{SENTINEL}: 1\n{SENTINEL}: 2\n---\nBroken.\n"
    );
    write(root, &ticket_path(TICKET_B), &malformed);
    let misplaced = ticket_source(TICKET_C, "Misplaced", "");
    write(root, MISPLACED_PATH, &misplaced);
    refresh_completely(&enabled.service, root);
    Nonconforming {
        fixture,
        enabled,
        marker,
        malformed,
        misplaced,
    }
}

fn assert_nonconforming(item: &ItemDto, kind: ItemDtoKind, path: &str, code: ProblemCode) {
    assert_eq!(item.id, None, "{item:?}");
    assert_eq!(item.kind, kind);
    assert_eq!(item.path, path);
    assert_eq!(
        (
            &item.title,
            &item.ticket_type,
            &item.status,
            &item.project,
            &item.team
        ),
        (&None, &None, &None, &None, &None)
    );
    assert_eq!(item.closure, None);
    assert!(item.unknown_metadata.is_empty());
    assert_eq!(item.changed_at, None);
    assert_eq!(item.change_source, None);
    assert_eq!(item.problems.len(), 1, "{item:?}");
    assert_eq!(item.problems[0].code, code);
    assert_eq!(item.problems[0].path.as_deref(), Some(path));
    assert_eq!(item.context.kind, ItemContextKind::Primary);
}

#[test]
fn nonconforming_files_are_listed_with_null_ids_and_survive_every_filter() {
    let nonconforming = nonconforming_repository();
    let service = &nonconforming.enabled.service;
    let repo = service
        .resolve_repository(&nonconforming.fixture.root)
        .unwrap();
    let malformed_path = ticket_path(TICKET_B);

    let documents = service.list_documents(&repo).unwrap();
    let tickets = all_tickets(service, &repo);

    assert_eq!(
        paths(&documents),
        ["docs/a.md", MARKER_PATH, MISPLACED_PATH]
    );
    assert_eq!(ids(&documents), [Some(DOCUMENT_A), None, None]);
    assert_nonconforming(
        &documents.items[1],
        ItemDtoKind::Document,
        MARKER_PATH,
        ProblemCode::MissingField,
    );
    assert_nonconforming(
        &documents.items[2],
        ItemDtoKind::Document,
        MISPLACED_PATH,
        ProblemCode::KindPathMismatch,
    );
    // The ticket filed under docs/ is not a ticket anywhere.
    assert_eq!(ids(&tickets), [Some(TICKET_A), None]);
    assert_nonconforming(
        &tickets.items[1],
        ItemDtoKind::Ticket,
        &malformed_path,
        ProblemCode::MalformedFrontMatter,
    );
    for item in documents.items.iter().chain(&tickets.items) {
        assert_list_form(item);
        assert_eq!(item.context.worktree, root_string(&nonconforming.fixture));
    }

    let text = |value: &str| Some(value.to_owned());
    for filter in [
        TicketFilter {
            status: text("absent"),
            ..Default::default()
        },
        TicketFilter {
            ticket_type: text("absent"),
            ..Default::default()
        },
        TicketFilter {
            project: text("absent"),
            ..Default::default()
        },
        TicketFilter {
            closure: ClosureFilter::Closed,
            ..Default::default()
        },
        TicketFilter {
            status: text("absent"),
            ticket_type: text("absent"),
            project: text("absent"),
            closure: ClosureFilter::Closed,
            slug: text("absent"),
            readiness: Some(ReadinessFilter::Ready),
        },
    ] {
        let filtered = service.list_tickets(&repo, &filter).unwrap();
        assert_eq!(ids(&filtered), [None], "{filter:?}");
        assert_eq!(filtered.items[0], tickets.items[1]);
    }
    // A filter that keeps the ticket keeps the entry as well.
    let open = service
        .list_tickets(
            &repo,
            &TicketFilter {
                closure: ClosureFilter::Open,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&open), [Some(TICKET_A), None]);
    assert_git_transport_uninitialized();
}

#[test]
fn nonconforming_files_are_read_by_exact_path_with_their_source() {
    let nonconforming = nonconforming_repository();
    let service = &nonconforming.enabled.service;
    let repo = service
        .resolve_repository(&nonconforming.fixture.root)
        .unwrap();
    let malformed_path = ticket_path(TICKET_B);

    for (path, kind, code, source) in [
        (
            MARKER_PATH,
            ItemDtoKind::Document,
            ProblemCode::MissingField,
            &nonconforming.marker,
        ),
        (
            malformed_path.as_str(),
            ItemDtoKind::Ticket,
            ProblemCode::MalformedFrontMatter,
            &nonconforming.malformed,
        ),
        (
            MISPLACED_PATH,
            ItemDtoKind::Document,
            ProblemCode::KindPathMismatch,
            &nonconforming.misplaced,
        ),
    ] {
        let shown = service.show_path(&repo, None, Path::new(path)).unwrap();

        assert_nonconforming(&shown, kind, path, code);
        assert_eq!(shown.source.as_ref(), Some(source), "{path}");
        assert_eq!(shown.body, None);
        assert!(shown.observation.as_deref().unwrap().starts_with("v1:"));
        // The index already knows this file is not an item.
        assert_eq!(shown.index.state, IndexState::Current);
    }

    // A conforming item is read by path as well, and is the same read.
    let by_path = service
        .show_path(&repo, None, Path::new("docs/a.md"))
        .unwrap();
    let by_id = service.show_item(&repo, &item_id(DOCUMENT_A)).unwrap();
    assert!(by_path == by_id);
    assert_eq!(by_path.id.as_deref(), Some(DOCUMENT_A));
    assert_eq!(by_path.index.state, IndexState::Current);
    assert_git_transport_uninitialized();
}

#[test]
fn a_parser_message_stored_with_a_problem_is_never_published() {
    let nonconforming = nonconforming_repository();
    let service = &nonconforming.enabled.service;
    let repo = service
        .resolve_repository(&nonconforming.fixture.root)
        .unwrap();
    // The sentinel really is in the index, so the scan below can fail.
    let stored: String = index(nonconforming.enabled.data_directory.path())
        .query_row(
            "SELECT guidance FROM problems WHERE code = 'malformed-front-matter'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(stored.contains(SENTINEL), "{stored}");

    let tickets = all_tickets(service, &repo);
    let envelope = serde_json::to_value(Envelope::read_success(
        "ticket list",
        repo.scope(),
        tickets.clone(),
    ))
    .unwrap();

    assert_eq!(support::golden::find_sentinel(&envelope, &[SENTINEL]), None);
    let problem = &envelope["data"]["items"][1]["problems"][0];
    assert_eq!(problem["code"], "malformed_front_matter");
    assert_eq!(problem["guidance"], "Correct the YAML front matter.");
    assert_eq!(
        tickets.items[1].problems[0].guidance(),
        ProblemCode::MalformedFrontMatter.guidance()
    );
    // Read by path, the file's own text is returned and nothing else is.
    let shown = serde_json::to_value(
        service
            .show_path(&repo, None, Path::new(&ticket_path(TICKET_B)))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(shown["source"], nonconforming.malformed.as_str());
    assert_eq!(
        shown["problems"][0]["guidance"],
        "Correct the YAML front matter."
    );
    let mut without_source = shown.clone();
    without_source["source"] = Value::Null;
    assert_eq!(
        support::golden::find_sentinel(&without_source, &[SENTINEL]),
        None
    );
    assert_git_transport_uninitialized();
}

#[test]
fn only_a_conformity_problem_at_an_item_path_with_no_item_makes_an_entry() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "A", ""),
    );
    // A directory where an item worktree would be: a context problem.
    fs::create_dir_all(root.join(".manyhands/worktrees").join(DOCUMENT_B)).unwrap();
    // A managed file outside every canonical location: its path is invalid.
    write(
        root,
        "notes/stray.md",
        &document_source(DOCUMENT_C, "Stray", ""),
    );
    // A comment that is not one.
    write(
        root,
        &format!(".manyhands/comments/{DOCUMENT_A}/{TICKET_D}.md"),
        "---\nmanyhands_managed: true\n---\n",
    );
    // The primary branch is not checked out: a branch problem, with no path.
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch("other", &head, false).unwrap();
    fixture.repository.set_head("refs/heads/other").unwrap();
    refresh_completely(&enabled.service, root);
    let connection = index(enabled.data_directory.path());
    let stored: Vec<(Option<String>, String)> = connection
        .prepare("SELECT path, code FROM problems ORDER BY code, path")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        stored,
        [
            (None, "branch".to_owned()),
            (
                Some(format!(".manyhands/worktrees/{DOCUMENT_B}")),
                "context".to_owned()
            ),
            (Some("notes/stray.md".to_owned()), "invalid-path".to_owned()),
            (
                Some(format!(".manyhands/comments/{DOCUMENT_A}/{TICKET_D}.md")),
                "missing-field".to_owned()
            ),
        ]
    );
    // What no scan stores, but the rule must still refuse: a conformity
    // code with no path, one with no context, one at a listed item's own
    // path, and a code that is not about conformity at an item path.
    let (repository_id, context_id): (i64, i64) = connection
        .query_row("SELECT repository_id, id FROM contexts", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    for (context, path, code) in [
        (Some(context_id), None, "missing-field"),
        (None, Some("docs/no-context.md"), "missing-field"),
        (Some(context_id), Some("docs/a.md"), "duplicate-id"),
        (Some(context_id), Some("docs/unreadable.md"), "source"),
        (Some(context_id), Some("docs/retry.md"), "retry-required"),
        (
            Some(context_id),
            Some("docs/config.md"),
            "malformed-configuration",
        ),
        (Some(context_id), Some("docs/comment.md"), "missing-parent"),
        (
            Some(context_id),
            Some("docs/future.md"),
            "a-code-from-the-future",
        ),
        (Some(context_id), Some("docsx/near.md"), "missing-field"),
        (
            Some(context_id),
            Some(".manyhands/ticketsx/near.md"),
            "missing-field",
        ),
    ] {
        connection
            .execute(
                "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
                 VALUES (?1, ?2, ?3, ?4, 'stored guidance', 1)",
                rusqlite::params![repository_id, context, path, code],
            )
            .unwrap();
    }
    drop(connection);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let documents = enabled.service.list_documents(&repo).unwrap();
    let tickets = all_tickets(&enabled.service, &repo);

    assert_eq!(ids(&documents), [Some(DOCUMENT_A)]);
    assert_eq!(ids(&tickets), [Some(TICKET_A)]);
    // The root could not be verified as the primary context, and says so.
    assert_eq!(documents.items[0].context.kind, ItemContextKind::Unverified);
    assert_eq!(documents.items[0].context.branch.as_deref(), Some("other"));
    assert!(documents.items[0].problems.is_empty());
    assert!(enabled.service.list_repositories().unwrap().items[0].problem_count >= 14);

    // Each of the seven conformity codes does make an entry, in the list
    // its path belongs to, and one file with two problems is one entry.
    let connection = index(enabled.data_directory.path());
    for (path, code) in [
        ("docs/1.md", "missing-front-matter"),
        ("docs/2.md", "malformed-front-matter"),
        ("docs/3.md", "missing-field"),
        ("docs/4.md", "invalid-field"),
        ("docs/5.md", "kind-path-mismatch"),
        ("docs/6.md", "invalid-path"),
        ("docs/7.md", "duplicate-id"),
        ("docs/7.md", "invalid-field"),
        (".manyhands/tickets/x/ticket.md", "invalid-path"),
    ] {
        connection
            .execute(
                "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
                 VALUES (?1, ?2, ?3, ?4, 'stored guidance', 1)",
                rusqlite::params![repository_id, context_id, path, code],
            )
            .unwrap();
    }
    drop(connection);
    let documents = enabled.service.list_documents(&repo).unwrap();
    let tickets = all_tickets(&enabled.service, &repo);
    assert_eq!(
        documents
            .items
            .iter()
            .map(|item| (item.path.as_str(), codes(item)))
            .collect::<Vec<_>>(),
        [
            ("docs/1.md", vec![ProblemCode::MissingFrontMatter]),
            ("docs/2.md", vec![ProblemCode::MalformedFrontMatter]),
            ("docs/3.md", vec![ProblemCode::MissingField]),
            ("docs/4.md", vec![ProblemCode::InvalidField]),
            ("docs/5.md", vec![ProblemCode::KindPathMismatch]),
            ("docs/6.md", vec![ProblemCode::InvalidPath]),
            (
                "docs/7.md",
                vec![ProblemCode::DuplicateId, ProblemCode::InvalidField]
            ),
            ("docs/a.md", vec![]),
        ]
    );
    assert_eq!(
        paths(&tickets),
        [
            ticket_path(TICKET_A).as_str(),
            ".manyhands/tickets/x/ticket.md"
        ]
    );
    let envelope = serde_json::to_value(&documents).unwrap();
    assert_eq!(
        support::golden::find_sentinel(&envelope, &["stored guidance"]),
        None
    );
    assert_git_transport_uninitialized();
}

#[test]
fn unknown_metadata_is_json_and_a_value_with_no_json_form_is_null_and_reported() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let source = ticket_source(
        TICKET_A,
        "Odd",
        "zeta: last\ntagged: !secret hidden\nnested:\n  deep: [1, 2.5, true]\n\
         not_a_number: .nan\n",
    );
    let path = ticket_path(TICKET_A);
    write(root, &path, &source);
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "Plain", "extra: kept\n"),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let expected = json!({
        "nested": {"deep": [1, 2.5, true]},
        "not_a_number": null,
        "tagged": null,
        "zeta": "last",
    });

    let list = all_tickets(&enabled.service, &repo);
    let listed = list
        .items
        .iter()
        .find(|item| item.id.as_deref() == Some(TICKET_A))
        .unwrap();
    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    let plain = enabled
        .service
        .show_item(&repo, &item_id(TICKET_B))
        .unwrap();

    for item in [listed, &shown] {
        assert_eq!(Value::Object(item.unknown_metadata.clone()), expected);
        // Keys are in order, whatever order the file has them in.
        assert_eq!(
            item.unknown_metadata.keys().collect::<Vec<_>>(),
            ["nested", "not_a_number", "tagged", "zeta"]
        );
        assert_eq!(codes(item), [ProblemCode::MetadataNotRepresentable]);
        assert_eq!(item.problems[0].path.as_deref(), Some(path.as_str()));
        // The fields Manyhands defines are not unknown.
        assert_eq!(item.title.as_deref(), Some("Odd"));
    }
    // The file's own text still has the value exactly.
    assert_eq!(shown.source.as_deref(), Some(source.as_str()));
    assert!(shown.source.unwrap().contains("tagged: !secret hidden\n"));
    assert_eq!(shown.index.state, IndexState::Current);
    assert_eq!(
        Value::Object(plain.unknown_metadata),
        json!({"extra": "kept"})
    );
    assert!(plain.problems.is_empty());
    assert_git_transport_uninitialized();
}

#[test]
fn the_observation_token_follows_the_source_and_is_otherwise_stable() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let source = document_source(DOCUMENT_A, "A", "");
    let file = write(root, "docs/a.md", &source);
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "A", ""),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let token = || {
        enabled
            .service
            .show_item(&repo, &item_id(DOCUMENT_A))
            .unwrap()
            .observation
            .unwrap()
    };

    let first = token();

    // "v1:" and a BLAKE3 digest in lowercase hexadecimal.
    assert_eq!(first.len(), 3 + 64);
    assert!(first.starts_with("v1:"));
    assert!(
        first[3..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_eq!(token(), first);
    assert_eq!(
        enabled
            .service
            .show_path(&repo, None, Path::new("docs/a.md"))
            .unwrap()
            .observation
            .unwrap(),
        first
    );
    // A refresh, or a rebuild, of unchanged content changes nothing.
    refresh_completely(&enabled.service, root);
    rebuild(&enabled.service, root);
    assert_eq!(token(), first);
    assert_ne!(
        enabled
            .service
            .show_item(&repo, &item_id(TICKET_A))
            .unwrap()
            .observation
            .unwrap(),
        first
    );

    // One byte of the body.
    fs::write(&file, format!("{source} ")).unwrap();
    let changed = token();
    assert_ne!(changed, first);
    fs::write(&file, &source).unwrap();
    assert_eq!(token(), first);

    // The same bytes at another path are another observation.
    fs::rename(&file, root.join("docs/b.md")).unwrap();
    refresh_completely(&enabled.service, root);
    assert_ne!(token(), first);
    assert_git_transport_uninitialized();
}

/// What tells one list result from another: whether it failed, its code,
/// and for a success the index state and the number of entries.
fn list_result(
    result: &Result<ItemListDto, ReadError>,
) -> (Outcome, ResultCode, Option<(IndexState, usize)>) {
    match result {
        Ok(list) => {
            let envelope =
                Envelope::read_success("document list", Default::default(), list.clone());
            (
                envelope.outcome,
                envelope.code,
                Some((list.index.state, list.items.len())),
            )
        }
        Err(error) => {
            let envelope = error.to_envelope::<Value>("document list");
            (envelope.outcome, envelope.code, None)
        }
    }
}

#[test]
fn empty_never_refreshed_stale_and_degraded_are_four_results_and_one_failure() {
    // Refreshed, and holding nothing.
    let (empty_fixture, empty_enabled) = enabled();
    let empty_repo = empty_enabled
        .service
        .resolve_repository(&empty_fixture.root)
        .unwrap();
    let empty = empty_enabled.service.list_documents(&empty_repo);

    // Registered, and never refreshed.
    let never = never_refreshed_repository();
    let never_repo = never
        .service
        .resolve_repository(&never.fixture.root)
        .unwrap();
    let never_refreshed = never.service.list_documents(&never_repo);

    // Refreshed, and since known to be behind.
    let (stale_fixture, stale_enabled) = enabled();
    let document = write(
        &stale_fixture.root,
        "docs/a.md",
        &document_source(DOCUMENT_A, "A", ""),
    );
    refresh_completely(&stale_enabled.service, &stale_fixture.root);
    stale_enabled
        .service
        .set_observation_hook_for_testing(move || fs::write(document, "plain\n").unwrap());
    assert!(matches!(
        refresh(&stale_enabled.service, &stale_fixture.root),
        RefreshOutcome::RetryRequired { .. }
    ));
    let stale_repo = stale_enabled
        .service
        .resolve_repository(&stale_fixture.root)
        .unwrap();
    let stale = stale_enabled.service.list_documents(&stale_repo);

    // An index that cannot be read.
    let (degraded_fixture, degraded_enabled) = enabled();
    write(
        &degraded_fixture.root,
        "docs/a.md",
        &document_source(DOCUMENT_A, "A", ""),
    );
    refresh_completely(&degraded_enabled.service, &degraded_fixture.root);
    let degraded_repo = degraded_enabled
        .service
        .resolve_repository(&degraded_fixture.root)
        .unwrap();
    let (_data, degraded_service) = degraded_service(degraded_enabled);
    let degraded = degraded_service.list_documents(&degraded_repo);

    assert_eq!(
        list_result(&empty),
        (
            Outcome::Success,
            ResultCode::Ok,
            Some((IndexState::Current, 0))
        )
    );
    assert_eq!(
        list_result(&never_refreshed),
        (
            Outcome::Success,
            ResultCode::Ok,
            Some((IndexState::NeverRefreshed, 0))
        )
    );
    assert_eq!(
        list_result(&stale),
        (
            Outcome::Success,
            ResultCode::Ok,
            Some((IndexState::Stale, 1))
        )
    );
    assert_eq!(
        list_result(&degraded),
        (Outcome::Blocked, ResultCode::IndexUnavailable, None)
    );
    assert!(empty.as_ref().unwrap().index.refreshed_at.is_some());
    assert_eq!(never_refreshed.as_ref().unwrap().index.refreshed_at, None);
    assert!(stale.as_ref().unwrap().index.refreshed_at.is_some());
    // The same four for tickets.
    assert_eq!(
        all_tickets(&never.service, &never_repo).index.state,
        IndexState::NeverRefreshed
    );
    assert_eq!(
        all_tickets(&stale_enabled.service, &stale_repo).index.state,
        IndexState::Stale
    );

    // Every item read of a degraded index fails the same way, names the
    // repository and says which root to rebuild.
    let root = root_string(&degraded_fixture);
    let failures = [
        degraded.unwrap_err(),
        degraded_service
            .list_tickets(&degraded_repo, &TicketFilter::default())
            .unwrap_err(),
        degraded_service
            .show_item(&degraded_repo, &item_id(DOCUMENT_A))
            .unwrap_err(),
        degraded_service
            .show_path(&degraded_repo, None, Path::new("docs/a.md"))
            .unwrap_err(),
    ];
    for error in &failures {
        assert_eq!(error.code(), ResultCode::IndexUnavailable);
        assert_eq!(error.scope.repository.as_deref(), Some(root.as_str()));
        assert_eq!(
            recovery(error),
            json!([{"action": "index.rebuild", "operation_id": null, "arguments": {"root": root}}])
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn an_item_whose_file_is_gone_is_not_found_and_a_refresh_is_the_recovery() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    let ticket = write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "A", ""),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let root_text = root_string(&fixture);
    let assert_gone = |id: &str| {
        let error = enabled.service.show_item(&repo, &item_id(id)).unwrap_err();
        assert_eq!(error.code(), ResultCode::ItemNotFound, "{id}");
        assert_eq!(error.scope.item_id.as_deref(), Some(id));
        assert_eq!(error.scope.repository.as_deref(), Some(root_text.as_str()));
        assert_eq!(recovery(&error), refresh_recovery(&root_text));
        assert_eq!(
            error.to_envelope::<Value>("document show").outcome,
            Outcome::Error
        );
    };

    // The file is deleted.
    fs::remove_file(&file).unwrap();
    assert_gone(DOCUMENT_A);
    // The directory that held the file is deleted.
    fs::remove_dir_all(ticket.parent().unwrap()).unwrap();
    assert_gone(TICKET_A);
    // Another item's file is where this one's was.
    fs::write(&file, document_source(DOCUMENT_B, "Another", "")).unwrap();
    assert_gone(DOCUMENT_A);
    // The index still lists what it stored.
    assert_eq!(
        ids(&enabled.service.list_documents(&repo).unwrap()),
        [Some(DOCUMENT_A)]
    );

    // An ID the index has never held: nothing a refresh of a current index
    // would find.
    let unknown = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_C))
        .unwrap_err();
    assert_eq!(unknown.code(), ResultCode::ItemNotFound);
    assert_eq!(unknown.scope.item_id.as_deref(), Some(DOCUMENT_C));
    assert_eq!(recovery(&unknown), json!([]));
    // A comment's ID is not an item's.
    refresh_completely(&enabled.service, root);
    let after = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();
    assert_eq!(after.code(), ResultCode::ItemNotFound);
    assert_eq!(recovery(&after), json!([]));
    assert_git_transport_uninitialized();
}

#[test]
fn an_unknown_id_in_an_index_that_is_behind_suggests_a_refresh() {
    let never = never_refreshed_repository();
    let repo = never
        .service
        .resolve_repository(&never.fixture.root)
        .unwrap();

    let error = never
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::ItemNotFound);
    assert_eq!(
        recovery(&error),
        refresh_recovery(&root_string(&never.fixture))
    );
    assert_git_transport_uninitialized();
}

#[test]
fn an_indexed_item_that_no_longer_parses_is_shown_as_nonconforming_with_its_source() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let path = ticket_path(TICKET_A);
    let file = write(
        root,
        &path,
        &ticket_source(TICKET_A, "A", "project: alpha\n"),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let broken = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\n---\nNo longer a ticket.\n";
    fs::write(&file, broken).unwrap();

    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();

    assert_eq!(shown.id, None);
    assert_eq!(shown.kind, ItemDtoKind::Ticket);
    assert_eq!(shown.path, path);
    assert_eq!(shown.title, None);
    assert_eq!(shown.project, None);
    assert_eq!(shown.closure, None);
    assert_eq!(shown.body, None);
    assert_eq!(shown.source.as_deref(), Some(broken));
    assert!(shown.observation.is_some());
    assert_eq!(shown.changed_at, None);
    assert_eq!(codes(&shown), [ProblemCode::MissingField]);
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_eq!(shown.context.kind, ItemContextKind::Primary);

    // Not text at all: there is no source to give.
    fs::write(&file, [0xff, 0xfe, 0x00]).unwrap();
    let binary = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    assert_eq!(binary.id, None);
    assert_eq!(binary.source, None);
    assert_eq!(codes(&binary), [ProblemCode::SourceUnreadable]);
    assert_eq!(binary.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn show_path_reads_only_canonical_item_paths() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write(root, "docs/notes.txt", "not markdown\n");
    write(root, "README.md", "readme\n");
    write(
        root,
        "docs/sub/deep/b.md",
        &document_source(DOCUMENT_B, "B", ""),
    );
    let comment = format!(".manyhands/comments/{DOCUMENT_A}/{TICKET_D}.md");
    write(root, &comment, "---\nmanyhands_managed: true\n---\n");
    write(
        root,
        &format!(".manyhands/tickets/{TICKET_A}/notes.md"),
        "notes\n",
    );
    write(root, ".manyhands/tickets/not-an-id/ticket.md", "---\n---\n");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let absolute = fs::canonicalize(root.join("docs/a.md")).unwrap();
    let show = |path: &str| enabled.service.show_path(&repo, None, Path::new(path));

    for path in [
        absolute.to_str().unwrap(),
        "/etc/hostname",
        "../outside.md",
        "docs/../docs/a.md",
        "docs/../../outside.md",
        "docs/sub/../a.md",
        "./docs/a.md",
        "docs/./a.md",
        "docs//a.md",
        "docs/a.md/",
        // Through a file, as if it were a directory.
        "docs/a.md/b.md",
        "",
        ".",
        "docs",
        "docs/",
        "docs/notes.txt",
        "docs/a",
        "README.md",
        "fixture.txt",
        ".manyhands/config.toml",
        ".git/config",
        ".git/HEAD",
        comment.as_str(),
        &format!(".manyhands/tickets/{TICKET_A}/notes.md"),
        &format!(".manyhands/tickets/{TICKET_A}"),
        &format!(".manyhands/tickets/{}/ticket.md", TICKET_A.to_lowercase()),
        &format!(".manyhands/worktrees/{DOCUMENT_A}/docs/a.md"),
        "docs\\a.md",
        "Docs/a.md",
    ] {
        let error = show(path).unwrap_err();
        assert_eq!(error.code(), ResultCode::InvalidPath, "{path:?}");
        assert_eq!(recovery(&error), json!([]));
        assert_eq!(
            error.scope.repository.as_deref(),
            Some(root_string(&fixture).as_str())
        );
    }

    // A canonical path with nothing at it, or under a directory that is
    // not there.
    for path in ["docs/absent.md", "docs/absent/b.md", &ticket_path(TICKET_B)] {
        let error = show(path).unwrap_err();
        assert_eq!(error.code(), ResultCode::PathNotFound, "{path:?}");
        assert_eq!(recovery(&error), json!([]));
    }
    assert_eq!(show("docs/a.md").unwrap().id.as_deref(), Some(DOCUMENT_A));
    // A path that is not an item's is read when, and only when, a list
    // shows it as a nonconforming entry.
    let listed = all_tickets(&enabled.service, &repo);
    assert_eq!(paths(&listed), [".manyhands/tickets/not-an-id/ticket.md"]);
    let shown = show(".manyhands/tickets/not-an-id/ticket.md").unwrap();
    assert_eq!(shown.id, None);
    assert_eq!(shown.kind, ItemDtoKind::Ticket);
    assert_eq!(shown.source.as_deref(), Some("---\n---\n"));
    assert_eq!(codes(&shown), [ProblemCode::InvalidPath]);
    assert_eq!(shown.index.state, IndexState::Current);
    assert_eq!(shown.problems, listed.items[0].problems);
    assert_eq!(
        show("docs/sub/deep/b.md").unwrap().id.as_deref(),
        Some(DOCUMENT_B)
    );
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn show_path_follows_no_symbolic_link() {
    use std::os::unix::fs::symlink;

    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let outside = tempfile::tempdir().unwrap();
    let elsewhere = document_source(DOCUMENT_B, "Outside", "");
    write(outside.path(), "secret.md", &elsewhere);
    write(outside.path(), "directory/inner.md", &elsewhere);
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    // A file that is a link out of the repository, a directory that is one,
    // a link to a file inside the repository, and a ticket directory that
    // is a link.
    symlink(outside.path().join("secret.md"), root.join("docs/link.md")).unwrap();
    symlink(outside.path().join("directory"), root.join("docs/linked")).unwrap();
    symlink(root.join("docs/a.md"), root.join("docs/inside.md")).unwrap();
    fs::create_dir_all(root.join(".manyhands/tickets")).unwrap();
    symlink(
        outside.path().join("directory"),
        root.join(".manyhands/tickets").join(TICKET_A),
    )
    .unwrap();
    write(outside.path(), "directory/ticket.md", &elsewhere);
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    for path in [
        "docs/link.md".to_owned(),
        "docs/linked/inner.md".to_owned(),
        "docs/inside.md".to_owned(),
        ticket_path(TICKET_A),
    ] {
        let error = enabled
            .service
            .show_path(&repo, None, Path::new(&path))
            .unwrap_err();

        assert_eq!(error.code(), ResultCode::InvalidPath, "{path}");
    }
    // No list has an entry that would point a caller at them either.
    let documents = enabled.service.list_documents(&repo).unwrap();
    assert_eq!(paths(&documents), ["docs/a.md"]);
    assert!(all_tickets(&enabled.service, &repo).items.is_empty());
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn show_item_does_not_follow_a_link_that_replaced_an_indexed_file() {
    use std::os::unix::fs::symlink;

    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let outside = tempfile::tempdir().unwrap();
    // The same item, by ID, kept outside the repository.
    let elsewhere = document_source(DOCUMENT_A, "A", "");
    write(outside.path(), "secret.md", &elsewhere);
    let file = write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    fs::remove_file(&file).unwrap();
    symlink(outside.path().join("secret.md"), &file).unwrap();

    let error = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::ItemNotFound);
    assert_eq!(recovery(&error), refresh_recovery(&root_string(&fixture)));
    assert_git_transport_uninitialized();
}

#[test]
fn show_path_reads_the_primary_context_unless_an_active_one_is_named() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(
        root,
        "docs/shared.md",
        &document_source(DOCUMENT_C, "Shared", ""),
    );
    commit(&fixture, &["docs/shared.md"], 1_000);
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/active.md");
    let worktree = context_worktree(root, DOCUMENT_A);
    // The worktree's copy of the shared document differs from the primary's.
    let stale_copy = document_source(DOCUMENT_C, "Old shared", "");
    fs::write(worktree.join("docs/shared.md"), &stale_copy).unwrap();
    // A linked worktree that is not an item's.
    let other = tempfile::tempdir().unwrap();
    let linked = other.path().join("linked");
    fixture
        .repository
        .worktree("linked", &linked, None)
        .unwrap();
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let show = |context: Option<&Path>, path: &str| {
        enabled.service.show_path(&repo, context, Path::new(path))
    };

    // By default, the primary context.
    let primary = show(None, "docs/shared.md").unwrap();
    assert_eq!(primary.context.kind, ItemContextKind::Primary);
    assert_eq!(primary.title.as_deref(), Some("Shared"));
    assert_eq!(primary.index.state, IndexState::Current);
    assert!(primary.changed_at.is_some());
    assert!(show(Some(root), "docs/shared.md").unwrap() == primary);
    assert_eq!(
        show(None, "docs/active.md").unwrap_err().code(),
        ResultCode::PathNotFound
    );

    // In the named item worktree: its own item, read as `show_item` reads it.
    let active = show(Some(&worktree), "docs/active.md").unwrap();
    assert_eq!(active.context.kind, ItemContextKind::Active);
    assert_eq!(Path::new(&active.context.worktree), worktree);
    assert!(
        active
            == enabled
                .service
                .show_item(&repo, &item_id(DOCUMENT_A))
                .unwrap()
    );
    // And its copy of another item, which is not the effective copy: the
    // index holds no row for it there, so nothing about it is stale and
    // it has no change time.
    let copy = show(Some(&worktree), "docs/shared.md").unwrap();
    assert_eq!(copy.source.as_deref(), Some(stale_copy.as_str()));
    assert_eq!(copy.context, active.context);
    assert_eq!(copy.changed_at, None);
    assert_eq!(copy.change_source, None);
    assert_eq!(copy.index.state, IndexState::Current);
    assert_ne!(copy.observation, primary.observation);

    // A context is the root or one of the repository's item worktrees.
    for context in [
        linked.as_path(),
        other.path(),
        &root.join("docs"),
        &root.join(".manyhands/worktrees"),
        &root.join("absent"),
        Path::new("relative"),
    ] {
        assert_eq!(
            show(Some(context), "docs/shared.md").unwrap_err().code(),
            ResultCode::InvalidPath,
            "{context:?}"
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_file_the_index_has_not_seen_is_read_by_path_and_says_the_index_is_behind() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let source = document_source(DOCUMENT_A, "New", "");
    write(root, "docs/new.md", &source);
    write(root, "docs/broken.md", "no front matter\n");

    let new = enabled
        .service
        .show_path(&repo, None, Path::new("docs/new.md"))
        .unwrap();
    let broken = enabled
        .service
        .show_path(&repo, None, Path::new("docs/broken.md"))
        .unwrap();

    assert_eq!(new.id.as_deref(), Some(DOCUMENT_A));
    assert_eq!(new.source.as_deref(), Some(source.as_str()));
    assert_eq!(new.changed_at, None);
    assert_eq!(new.index.state, IndexState::Stale);
    assert_eq!(broken.id, None);
    assert_eq!(codes(&broken), [ProblemCode::MissingFrontMatter]);
    assert_eq!(broken.source.as_deref(), Some("no front matter\n"));
    assert_eq!(broken.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn a_duplicate_id_is_listed_without_an_id_and_shown_with_its_problem() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/one.md", &document_source(DOCUMENT_A, "One", ""));
    write(root, "docs/two.md", &document_source(DOCUMENT_A, "Two", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = enabled.service.list_documents(&repo).unwrap();
    let shown = enabled
        .service
        .show_path(&repo, None, Path::new("docs/two.md"))
        .unwrap();

    assert_eq!(paths(&list), ["docs/one.md", "docs/two.md"]);
    assert_eq!(ids(&list), [None, None]);
    for item in &list.items {
        assert_eq!(codes(item), [ProblemCode::DuplicateId]);
    }
    // The file itself parses, so a read by path has its metadata, and
    // still says why no list gives it an ID.
    assert_eq!(shown.id.as_deref(), Some(DOCUMENT_A));
    assert_eq!(shown.title.as_deref(), Some("Two"));
    assert_eq!(codes(&shown), [ProblemCode::DuplicateId]);
    assert_eq!(shown.index.state, IndexState::Current);
    assert_eq!(
        enabled
            .service
            .show_item(&repo, &item_id(DOCUMENT_A))
            .unwrap_err()
            .code(),
        ResultCode::ItemNotFound
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_repository_removed_since_it_was_resolved_is_not_registered() {
    let (fixture, enabled) = enabled();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    index(enabled.data_directory.path())
        .execute("DELETE FROM repositories", [])
        .unwrap();

    let error = enabled.service.list_documents(&repo).unwrap_err();

    assert_eq!(error.code(), ResultCode::RepositoryNotRegistered);
    assert_eq!(
        enabled
            .service
            .show_item(&repo, &item_id(DOCUMENT_A))
            .unwrap_err()
            .code(),
        ResultCode::RepositoryNotRegistered
    );
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn show_item_does_not_follow_a_link_that_replaced_an_item_worktree() {
    use std::os::unix::fs::symlink;

    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/active.md");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let worktree = context_worktree(root, DOCUMENT_A);
    enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();
    // The worktree's directory is now a link to where its files went.
    let outside = tempfile::tempdir().unwrap();
    let moved = outside.path().join("moved");
    fs::rename(&worktree, &moved).unwrap();
    symlink(&moved, &worktree).unwrap();
    assert!(worktree.join("docs/active.md").is_file());

    let error = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::ItemNotFound);
    assert_eq!(recovery(&error), refresh_recovery(&root_string(&fixture)));
    // Named as a context, the link is not one: it resolves elsewhere.
    assert_eq!(
        enabled
            .service
            .show_path(&repo, Some(&worktree), Path::new("docs/active.md"))
            .unwrap_err()
            .code(),
        ResultCode::InvalidPath
    );
    assert_git_transport_uninitialized();
}

#[test]
fn an_index_row_that_points_outside_the_repository_is_not_read_from() {
    let outside = tempfile::tempdir().unwrap();
    let outside_root = fs::canonicalize(outside.path()).unwrap();
    let elsewhere = document_source(DOCUMENT_A, "A", "");
    write(&outside_root, "docs/a.md", &elsewhere);
    write(&outside_root, "plain.md", &elsewhere);
    let show = |change: &dyn Fn(&Connection)| {
        let (fixture, enabled) = enabled();
        write(
            &fixture.root,
            "docs/a.md",
            &document_source(DOCUMENT_A, "A", ""),
        );
        write(&fixture.root, "plain.md", &elsewhere);
        refresh_completely(&enabled.service, &fixture.root);
        let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
        change(&index(enabled.data_directory.path()));
        (
            enabled.service.show_item(&repo, &item_id(DOCUMENT_A)),
            enabled
                .service
                .show_path(&repo, Some(&outside_root), Path::new("docs/a.md")),
        )
    };

    // Untouched, the fixture reads.
    assert!(show(&|_| {}).0.is_ok());
    // A context somewhere else, as the primary and as an item's.
    // Named by a caller, a root that is not this repository's is no context
    // at all; an item worktree that is not under the root is a row no read
    // trusts.
    for (kind, named) in [
        ("primary", ResultCode::InvalidPath),
        ("active", ResultCode::InternalError),
    ] {
        let (by_id, by_path) = show(&|connection| {
            connection
                .execute(
                    "UPDATE contexts SET worktree_path = ?1, kind = ?2",
                    rusqlite::params![outside_root.to_str(), kind],
                )
                .unwrap();
        });
        for (result, code) in [(by_id, ResultCode::InternalError), (by_path, named)] {
            let error = result.unwrap_err();
            assert_eq!(error.code(), code, "{kind}");
        }
    }
    // A path that is not an item's.
    for path in ["plain.md", "../plain.md", ".git/config", "docs/../plain.md"] {
        let (by_id, _) = show(&|connection| {
            connection
                .execute("UPDATE discovered_items SET canonical_path = ?1", [path])
                .unwrap();
        });
        assert_eq!(
            by_id.unwrap_err().code(),
            ResultCode::InternalError,
            "{path}"
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_root_the_index_has_not_observed_is_read_by_path_as_unverified() {
    let never = never_refreshed_repository();
    let root = &never.fixture.root;
    let source = document_source(DOCUMENT_A, "A", "");
    write(root, "docs/a.md", &source);
    let repo = never.service.resolve_repository(root).unwrap();

    let shown = never
        .service
        .show_path(&repo, None, Path::new("docs/a.md"))
        .unwrap();

    // Nothing has checked which branch the root is on.
    assert_eq!(shown.context.kind, ItemContextKind::Unverified);
    assert_eq!(shown.context.branch, None);
    assert_eq!(shown.context.head_oid, None);
    assert_eq!(shown.context.worktree, root_string(&never.fixture));
    assert_eq!(shown.source.as_deref(), Some(source.as_str()));
    assert_eq!(shown.index.state, IndexState::NeverRefreshed);
    assert_eq!(shown.index.refreshed_at, None);
    assert_git_transport_uninitialized();
}

/// Adds the context of item `worktree_id`'s worktree to the index, holding
/// item `id` at `path`,
/// as a refresh that stored that context and has not yet removed it leaves
/// it. Nothing is created on disk.
fn insert_item_worktree_row(
    data_directory: &Path,
    root: &str,
    worktree_id: &str,
    id: &str,
    path: &str,
) {
    let connection = index(data_directory);
    let repository_id: i64 = connection
        .query_row("SELECT id FROM repositories", [], |row| row.get(0))
        .unwrap();
    connection
        .execute(
            "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id)
             VALUES (?1, 'active', ?2, ?3, ?4)",
            rusqlite::params![
                repository_id,
                format!("manyhands/document/{worktree_id}"),
                Path::new(root)
                    .join(".manyhands/worktrees")
                    .join(worktree_id)
                    .to_str()
                    .unwrap(),
                worktree_id
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO discovered_items
                (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
             VALUES (?1, ?2, 'document', ?3, 'Worktree row', 5, 'git')",
            rusqlite::params![connection.last_insert_rowid(), id, path],
        )
        .unwrap();
}

#[test]
fn an_item_the_index_holds_twice_is_listed_once_and_read_from_the_copy_that_is_there() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let primary = document_source(DOCUMENT_A, "Primary", "");
    write(root, "docs/a.md", &primary);
    write(root, "docs/b.md", &document_source(DOCUMENT_B, "B", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let root_text = root_string(&fixture);
    // The item is in the primary context and in the row of an item worktree
    // that is not there: a refresh has stored one and not yet removed the
    // other.
    insert_item_worktree_row(
        enabled.data_directory.path(),
        &root_text,
        DOCUMENT_A,
        DOCUMENT_A,
        "docs/a.md",
    );
    let show = || enabled.service.show_item(&repo, &item_id(DOCUMENT_A));

    let list = enabled.service.list_documents(&repo).unwrap();
    let shown = show().unwrap();

    assert_eq!(ids(&list), [Some(DOCUMENT_A), Some(DOCUMENT_B)]);
    assert_eq!(list.items[0].title.as_deref(), Some("Primary"));
    assert_eq!(list.items[0].context.kind, ItemContextKind::Primary);
    // The index is in the middle of changing, and every entry says so.
    assert_eq!(list.index.state, IndexState::Stale);
    assert_eq!(list.items[1].index.state, IndexState::Stale);
    assert_eq!(shown.context.kind, ItemContextKind::Primary);
    assert_eq!(shown.source.as_deref(), Some(primary.as_str()));
    assert_eq!(shown.index.state, IndexState::Stale);
    // The other item is not held twice; read alone, nothing about it is behind.
    assert_eq!(
        enabled
            .service
            .show_item(&repo, &item_id(DOCUMENT_B))
            .unwrap()
            .index
            .state,
        IndexState::Current
    );

    // The worktree is there, with its own copy: that copy is the effective
    // one, and the index is still behind.
    let worktree = root.join(".manyhands/worktrees").join(DOCUMENT_A);
    let edited = document_source(DOCUMENT_A, "Worktree", "");
    write(&worktree, "docs/a.md", &edited);
    let list = enabled.service.list_documents(&repo).unwrap();
    let shown = show().unwrap();

    assert_eq!(ids(&list), [Some(DOCUMENT_A), Some(DOCUMENT_B)]);
    assert_eq!(list.items[0].title.as_deref(), Some("Worktree row"));
    assert_eq!(list.items[0].context.kind, ItemContextKind::Active);
    assert_eq!(list.index.state, IndexState::Stale);
    assert_eq!(shown.context.kind, ItemContextKind::Active);
    assert_eq!(
        Path::new(&shown.context.worktree),
        fs::canonicalize(&worktree).unwrap()
    );
    assert_eq!(shown.source.as_deref(), Some(edited.as_str()));
    assert_eq!(shown.index.state, IndexState::Stale);

    // The worktree is there and its copy is not: a list, which opens no
    // file, still names the worktree; a complete read falls back.
    fs::remove_file(worktree.join("docs/a.md")).unwrap();
    assert_eq!(
        enabled.service.list_documents(&repo).unwrap().items[0]
            .context
            .kind,
        ItemContextKind::Active
    );
    let shown = show().unwrap();
    assert_eq!(shown.context.kind, ItemContextKind::Primary);
    assert_eq!(shown.source.as_deref(), Some(primary.as_str()));
    assert_eq!(shown.index.state, IndexState::Stale);

    // Neither copy: not found, and a refresh is the recovery.
    fs::remove_file(root.join("docs/a.md")).unwrap();
    let error = show().unwrap_err();
    assert_eq!(error.code(), ResultCode::ItemNotFound);
    assert_eq!(recovery(&error), refresh_recovery(&root_text));
    assert_git_transport_uninitialized();
}

/// Sets a path's permission bits and puts back what they were when
/// dropped, so that a failing assertion leaves nothing unreadable behind.
#[cfg(unix)]
struct ModeGuard {
    path: PathBuf,
    mode: u32,
}

#[cfg(unix)]
impl ModeGuard {
    fn set(path: &Path, mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let before = fs::metadata(path).unwrap().permissions().mode();
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        Self {
            path: path.to_owned(),
            mode: before,
        }
    }
}

#[cfg(unix)]
impl Drop for ModeGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;

        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.mode));
    }
}

/// Whether permission bits stop this user from reading. They do not for a
/// user that may read anything, and then a test of a refusal has nothing
/// to observe: it says so and stops.
#[cfg(unix)]
fn permissions_bind_or_skip(test: &str, probe: &Path) -> bool {
    let _guard = ModeGuard::set(probe, 0o000);
    let bind = fs::read(probe).is_err();
    if !bind {
        eprintln!(
            "SKIPPED {test}: this user can read a file with mode 000, \
             so no open can be refused here"
        );
    }
    bind
}

#[cfg(unix)]
#[test]
fn a_file_or_directory_that_may_not_be_opened_is_inaccessible() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write(root, "docs/sub/b.md", &document_source(DOCUMENT_B, "B", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let directory = root.join("docs/sub");
    let assert_inaccessible = |id: &str, path: &str| {
        for error in [
            enabled.service.show_item(&repo, &item_id(id)).unwrap_err(),
            enabled
                .service
                .show_path(&repo, None, Path::new(path))
                .unwrap_err(),
        ] {
            assert_eq!(error.code(), ResultCode::RepositoryInaccessible, "{path}");
            assert_eq!(recovery(&error), json!([]));
            assert_eq!(
                error.to_envelope::<Value>("document show").outcome,
                Outcome::Blocked
            );
        }
    };
    if !permissions_bind_or_skip(
        "a_file_or_directory_that_may_not_be_opened_is_inaccessible",
        &file,
    ) {
        return;
    }

    {
        let _unreadable = ModeGuard::set(&file, 0o000);
        assert_inaccessible(DOCUMENT_A, "docs/a.md");
        // A list reads no file and is what it was.
        assert_eq!(
            ids(&enabled.service.list_documents(&repo).unwrap()),
            [Some(DOCUMENT_A), Some(DOCUMENT_B)]
        );
    }
    enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();

    {
        let _unreadable = ModeGuard::set(&directory, 0o000);
        assert_inaccessible(DOCUMENT_B, "docs/sub/b.md");
    }
    enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_B))
        .unwrap();
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn a_copy_in_the_items_worktree_that_cannot_be_read_is_not_hidden_behind_the_primary_copy() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let primary = write(
        root,
        "docs/a.md",
        &document_source(DOCUMENT_A, "Primary", ""),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    insert_item_worktree_row(
        enabled.data_directory.path(),
        &root_string(&fixture),
        DOCUMENT_A,
        DOCUMENT_A,
        "docs/a.md",
    );
    let worktree = root.join(".manyhands/worktrees").join(DOCUMENT_A);
    let copy = write(
        &worktree,
        "docs/a.md",
        &document_source(DOCUMENT_A, "Worktree", ""),
    );
    let show = || enabled.service.show_item(&repo, &item_id(DOCUMENT_A));
    assert_eq!(show().unwrap().title.as_deref(), Some("Worktree"));
    if !permissions_bind_or_skip(
        "a_copy_in_the_items_worktree_that_cannot_be_read_is_not_hidden_behind_the_primary_copy",
        &primary,
    ) {
        return;
    }

    for unreadable in [copy.as_path(), worktree.join("docs").as_path()] {
        let _unreadable = ModeGuard::set(unreadable, 0o000);
        let error = show().unwrap_err();
        assert_eq!(
            error.code(),
            ResultCode::RepositoryInaccessible,
            "{unreadable:?}"
        );
    }
    assert_eq!(show().unwrap().title.as_deref(), Some("Worktree"));
    assert_git_transport_uninitialized();
}

#[test]
fn a_broken_row_for_the_items_worktree_fails_the_read_instead_of_falling_back() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(
        root,
        "docs/a.md",
        &document_source(DOCUMENT_A, "Primary", ""),
    );
    write(root, "plain.md", "not an item\n");
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // The worktree's row names a path no item can be at.
    insert_item_worktree_row(
        enabled.data_directory.path(),
        &root_string(&fixture),
        DOCUMENT_A,
        DOCUMENT_A,
        "plain.md",
    );

    let error = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::InternalError);
    assert_git_transport_uninitialized();
}

#[test]
fn a_copy_of_an_item_in_another_items_worktree_is_never_the_effective_one() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let primary = document_source(DOCUMENT_A, "Primary", "");
    write(root, "docs/a.md", &primary);
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // Item B's worktree is there and holds a copy of item A, and the index
    // has a row for that copy.
    insert_item_worktree_row(
        enabled.data_directory.path(),
        &root_string(&fixture),
        DOCUMENT_B,
        DOCUMENT_A,
        "docs/a.md",
    );
    let other = root.join(".manyhands/worktrees").join(DOCUMENT_B);
    write(
        &other,
        "docs/a.md",
        &document_source(DOCUMENT_A, "Other worktree", ""),
    );

    let list = enabled.service.list_documents(&repo).unwrap();
    let shown = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();

    assert_eq!(ids(&list), [Some(DOCUMENT_A)]);
    assert_eq!(list.items[0].title.as_deref(), Some("Primary"));
    assert_eq!(list.items[0].context.kind, ItemContextKind::Primary);
    assert_eq!(list.index.state, IndexState::Stale);
    assert_eq!(shown.context.kind, ItemContextKind::Primary);
    assert_eq!(shown.source.as_deref(), Some(primary.as_str()));
    assert_eq!(shown.index.state, IndexState::Stale);

    // With no row for the primary copy, that copy in another item's worktree
    // is the only place the index knows the item to be. It is answered
    // from as a last resort, and the index says it is behind.
    index(enabled.data_directory.path())
        .execute(
            "DELETE FROM discovered_items WHERE context_id IN
                (SELECT id FROM contexts WHERE kind != 'active')",
            [],
        )
        .unwrap();
    let list = enabled.service.list_documents(&repo).unwrap();
    let shown = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap();

    assert_eq!(ids(&list), [Some(DOCUMENT_A)]);
    assert_eq!(list.items[0].context.kind, ItemContextKind::Active);
    assert_eq!(list.index.state, IndexState::Stale);
    assert_eq!(shown.context.kind, ItemContextKind::Active);
    assert_eq!(shown.title.as_deref(), Some("Other worktree"));
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_git_transport_uninitialized();
}

#[test]
fn a_nonconforming_entry_whose_path_cannot_be_read_by_path_is_still_listed() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let connection = index(enabled.data_directory.path());
    let (repository_id, context_id): (i64, i64) = connection
        .query_row("SELECT repository_id, id FROM contexts", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    // A file name with a backslash in it, as a Unix checkout can hold.
    let path = "docs/a\\b.md";
    connection
        .execute(
            "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
             VALUES (?1, ?2, ?3, 'invalid-path', 'stored guidance', 1)",
            rusqlite::params![repository_id, context_id, path],
        )
        .unwrap();
    drop(connection);

    let list = enabled.service.list_documents(&repo).unwrap();
    let error = enabled
        .service
        .show_path(&repo, None, Path::new(path))
        .unwrap_err();

    assert_eq!(paths(&list), [path]);
    assert_eq!(codes(&list.items[0]), [ProblemCode::InvalidPath]);
    assert_eq!(error.code(), ResultCode::InvalidPath);
    assert_git_transport_uninitialized();
}

#[test]
fn a_path_with_a_nul_is_invalid_and_a_name_too_long_to_exist_is_not_found() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let show = |path: &str| {
        enabled
            .service
            .show_path(&repo, None, Path::new(path))
            .unwrap_err()
            .code()
    };

    assert_eq!(show("docs/a\0b.md"), ResultCode::InvalidPath);
    assert_eq!(show("docs/a.md\0"), ResultCode::InvalidPath);
    assert_eq!(
        show(&format!("docs/{}.md", "n".repeat(300))),
        ResultCode::PathNotFound
    );
    assert_eq!(
        show(&format!("docs/{}/a.md", "n".repeat(300))),
        ResultCode::PathNotFound
    );
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn a_socket_where_an_item_file_was_is_not_a_file() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    fs::remove_file(&file).unwrap();
    let _listener = std::os::unix::net::UnixListener::bind(&file).unwrap();

    let by_id = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();
    let by_path = enabled
        .service
        .show_path(&repo, None, Path::new("docs/a.md"))
        .unwrap_err();

    assert_eq!(by_id.code(), ResultCode::ItemNotFound);
    assert_eq!(by_path.code(), ResultCode::InvalidPath);
    assert_git_transport_uninitialized();
}

#[test]
fn a_well_formed_path_no_list_shows_suggests_a_refresh_when_the_index_is_behind() {
    let never = never_refreshed_repository();
    let root = &never.fixture.root;
    write(root, ".manyhands/tickets/not-an-id/ticket.md", "---\n---\n");
    let repo = never.service.resolve_repository(root).unwrap();
    let show = || {
        never.service.show_path(
            &repo,
            None,
            Path::new(".manyhands/tickets/not-an-id/ticket.md"),
        )
    };

    // The index has not seen the file, so no list shows it yet.
    let error = show().unwrap_err();
    assert_eq!(error.code(), ResultCode::InvalidPath);
    assert_eq!(
        recovery(&error),
        refresh_recovery(&root_string(&never.fixture))
    );
    // A path that is not written as one gets no such advice.
    let malformed = never
        .service
        .show_path(&repo, None, Path::new("../ticket.md"))
        .unwrap_err();
    assert_eq!(recovery(&malformed), json!([]));

    // Refreshed, the list shows it and it reads.
    assert!(matches!(
        refresh_as(&never.service, root, never.operation_id),
        RefreshOutcome::Refreshed { .. }
    ));
    assert_eq!(show().unwrap().id, None);
    assert_git_transport_uninitialized();
}

#[cfg(unix)]
#[test]
fn a_pipe_where_an_item_file_was_is_refused_without_waiting() {
    use std::os::unix::ffi::OsStrExt;

    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    fs::remove_file(&file).unwrap();
    let name = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let started = std::time::Instant::now();

    let by_id = enabled
        .service
        .show_item(&repo, &item_id(DOCUMENT_A))
        .unwrap_err();
    let by_path = enabled
        .service
        .show_path(&repo, None, Path::new("docs/a.md"))
        .unwrap_err();

    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(by_id.code(), ResultCode::ItemNotFound);
    assert_eq!(by_path.code(), ResultCode::InvalidPath);
    assert_git_transport_uninitialized();
}

#[test]
fn refreshed_at_is_the_time_a_refresh_or_rebuild_began_observing() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let data = enabled.data_directory.path();
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    // Runs between the two observations of one refresh, changes nothing,
    // and takes more than a second.
    let pause = |observed: &std::sync::Arc<std::sync::Mutex<i64>>| {
        let observed = observed.clone();
        move || {
            *observed.lock().unwrap() = now();
            std::thread::sleep(Duration::from_millis(1_100));
        }
    };

    let during_refresh = std::sync::Arc::new(std::sync::Mutex::new(0));
    enabled
        .service
        .set_observation_hook_for_testing(pause(&during_refresh));
    refresh_completely(&enabled.service, root);
    let [Some(refreshed)] = refreshed_at(data)[..] else {
        panic!("a completed refresh records its time");
    };
    let finished = now();

    let during_rebuild = std::sync::Arc::new(std::sync::Mutex::new(0));
    enabled
        .service
        .set_observation_hook_for_testing(pause(&during_rebuild));
    rebuild(&enabled.service, root);
    let [Some(rebuilt)] = refreshed_at(data)[..] else {
        panic!("a completed rebuild records its time");
    };

    // Not the time it finished, which is at least a second later.
    assert!(refreshed <= *during_refresh.lock().unwrap());
    assert!(refreshed < finished, "{refreshed} {finished}");
    assert!(rebuilt <= *during_rebuild.lock().unwrap());
    assert!(rebuilt >= finished);
    assert_git_transport_uninitialized();
}

#[test]
fn a_file_read_by_path_that_is_newer_than_the_refresh_is_stale_whatever_it_holds() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(
        root,
        "docs/shared.md",
        &document_source(DOCUMENT_C, "C", ""),
    );
    commit(&fixture, &["docs/shared.md"], 1_000);
    create_document_context(&enabled.service, root, DOCUMENT_A, "docs/active.md");
    let marker = write(root, MARKER_PATH, "---\nmanyhands_managed: true\n---\n");
    write(root, "docs/one.md", &document_source(DOCUMENT_B, "One", ""));
    let duplicate = write(root, "docs/two.md", &document_source(DOCUMENT_B, "Two", ""));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let worktree = context_worktree(root, DOCUMENT_A);
    let copy = worktree.join("docs/shared.md");
    let state = |context: Option<&Path>, path: &str| {
        enabled
            .service
            .show_path(&repo, context, Path::new(path))
            .unwrap()
            .index
            .state
    };
    // A nonconforming file, a duplicate, and a copy that is not the
    // effective one: the index is right about each of them.
    assert_eq!(state(None, MARKER_PATH), IndexState::Current);
    assert_eq!(state(None, "docs/two.md"), IndexState::Current);
    assert_eq!(
        state(Some(&worktree), "docs/shared.md"),
        IndexState::Current
    );

    for file in [&marker, &duplicate, &copy] {
        set_modified_later(file);
    }

    assert_eq!(state(None, MARKER_PATH), IndexState::Stale);
    assert_eq!(state(None, "docs/two.md"), IndexState::Stale);
    assert_eq!(state(Some(&worktree), "docs/shared.md"), IndexState::Stale);
    assert_git_transport_uninitialized();
}

// ---------------------------------------------------------------------
// Ticket relationships: the short code, the parent and the dependencies.
// ---------------------------------------------------------------------

use manyhands::{
    canonical::{CanonicalItem, TicketRelationships, parse_item, ticket_relationships},
    repository::{
        AuthoringKind, AuthoringTarget, ContextIntent, DependencyDto, DependencyState,
        ExpectedPathObservation, SaveOutcome, SaveTicketRequest, TicketDraft,
    },
};

/// An ID no fixture gives to an item.
const ABSENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZZ";
const SLUG: &str = "mh-vh-k9x2b";

/// Three tickets and a document:
///
/// - A has a short code, B as its parent, and C, B and an absent ticket as
///   its dependencies, written in flow form, beside an unknown key;
/// - B is closed and relates to nothing;
/// - C has one valid dependency, on A, and one of each invalid value;
/// - the document carries the same keys, which mean nothing on a document.
fn write_related_items(root: &Path) -> Vec<PathBuf> {
    vec![
        write(
            root,
            &ticket_path(TICKET_A),
            &ticket_source(
                TICKET_A,
                "A",
                &format!(
                    "slug: {SLUG}\nparent: {TICKET_B}\n\
                     deps: [{TICKET_C}, \"{TICKET_B}\", {ABSENT}]\nother: kept\n"
                ),
            ),
        ),
        write(
            root,
            &ticket_path(TICKET_B),
            &ticket_source(TICKET_B, "B", CLOSURE),
        ),
        write(
            root,
            &ticket_path(TICKET_C),
            &ticket_source(
                TICKET_C,
                "C",
                &format!(
                    "slug: Not-A-Slug\nparent: {TICKET_C}\n\
                     deps: [{TICKET_A}, {TICKET_A}, not-an-id, 7]\n"
                ),
            ),
        ),
        write(
            root,
            "docs/a.md",
            &document_source(
                DOCUMENT_A,
                "Document",
                &format!("slug: {SLUG}\ndeps: [{TICKET_A}]\n"),
            ),
        ),
    ]
}

/// What the index stores of relationships, by item ID so that two indexes
/// can be compared: `(item, slug)` for every item, `(item, target, kind)`
/// for every edge and `(item, code, detail)` for every item problem, each
/// in the order it was stored.
#[derive(Debug, PartialEq)]
struct StoredRelationships {
    slugs: Vec<(String, Option<String>)>,
    edges: Vec<(String, String, String)>,
    problems: Vec<(String, String, Option<String>)>,
}

fn stored_relationships(data_directory: &Path) -> StoredRelationships {
    let connection = index(data_directory);
    let slugs = connection
        .prepare("SELECT item_id, slug FROM discovered_items ORDER BY item_id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let edges = connection
        .prepare(
            "SELECT items.item_id, edges.target_id, edges.kind
               FROM item_edges AS edges JOIN discovered_items AS items ON items.id = edges.item_id
              ORDER BY items.item_id, edges.id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let problems = connection
        .prepare(
            "SELECT items.item_id, problems.code, problems.detail
               FROM item_problems AS problems
               JOIN discovered_items AS items ON items.id = problems.item_id
              ORDER BY items.item_id, problems.id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    StoredRelationships {
        slugs,
        edges,
        problems,
    }
}

/// Rows of `table` whose item row is gone.
fn orphans(data_directory: &Path, table: &str) -> i64 {
    index(data_directory)
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM {table}
                  WHERE item_id NOT IN (SELECT id FROM discovered_items)"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap()
}

fn expected_related_items() -> StoredRelationships {
    let text = |value: &str| value.to_owned();
    StoredRelationships {
        slugs: vec![
            (text(TICKET_A), Some(text(SLUG))),
            (text(TICKET_B), None),
            // A value that is not a short code is not stored as one.
            (text(TICKET_C), None),
            (text(DOCUMENT_A), None),
        ],
        edges: vec![
            (text(TICKET_A), text(TICKET_B), text("parent")),
            (text(TICKET_A), text(TICKET_C), text("deps")),
            (text(TICKET_A), text(TICKET_B), text("deps")),
            // Stored though no context holds its target.
            (text(TICKET_A), text(ABSENT), text("deps")),
            (text(TICKET_C), text(TICKET_A), text("deps")),
        ],
        problems: vec![
            (text(TICKET_C), text("invalid-slug"), None),
            (
                text(TICKET_C),
                text("relationship-self-reference"),
                Some(text(TICKET_C)),
            ),
            (
                text(TICKET_C),
                text("duplicate-dependency"),
                Some(text(TICKET_A)),
            ),
            (text(TICKET_C), text("relationship-invalid-id"), None),
            (text(TICKET_C), text("relationship-wrong-type"), None),
        ],
    }
}

#[test]
fn refresh_stores_short_codes_edges_and_relationship_problems_and_rebuild_stores_the_same() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write_related_items(root);

    refresh_completely(&enabled.service, root);

    let data = enabled.data_directory.path();
    let refreshed = stored_relationships(data);
    assert_eq!(refreshed, expected_related_items());
    // An ignored relationship value is not a problem with the file: the
    // problems that make nonconforming entries hold none of them.
    let file_problems: i64 = index(data)
        .query_row("SELECT COUNT(*) FROM problems", [], |row| row.get(0))
        .unwrap();
    assert_eq!(file_problems, 0);
    // The keys are fields of a ticket and unknown keys of a document.
    let unknown = stored_item_columns(data);
    let unknown_of = |id: &str| {
        unknown
            .iter()
            .find(|(item, _, _)| item == id)
            .and_then(|(_, _, unknown)| unknown.clone())
            .unwrap()
    };
    assert_eq!(
        unknown_of(TICKET_A),
        r#"{"not_representable":false,"values":{"other":"kept"}}"#
    );
    assert_eq!(
        unknown_of(TICKET_C),
        r#"{"not_representable":false,"values":{}}"#
    );
    assert_eq!(
        unknown_of(DOCUMENT_A),
        format!(
            r#"{{"not_representable":false,"values":{{"deps":["{TICKET_A}"],"slug":"{SLUG}"}}}}"#
        )
    );

    // Refreshing again replaces every row and leaves none behind.
    refresh_completely(&enabled.service, root);
    assert_eq!(stored_relationships(data), refreshed);
    // So does a rebuild, in this index and in one that held nothing.
    rebuild(&enabled.service, root);
    assert_eq!(stored_relationships(data), refreshed);
    let empty = tempfile::tempdir().unwrap();
    rebuild(&RepositoryService::open_at(empty.path()).unwrap(), root);
    assert_eq!(stored_relationships(empty.path()), refreshed);
    for directory in [data, empty.path()] {
        assert_eq!(orphans(directory, "item_edges"), 0);
        assert_eq!(orphans(directory, "item_problems"), 0);
    }
    assert_git_transport_uninitialized();
}

fn dependency(id: &str, state: DependencyState) -> DependencyDto {
    DependencyDto {
        id: id.to_owned(),
        state,
    }
}

/// The ID each of the item's problems is about.
fn target_ids(item: &ItemDto) -> Vec<Option<&str>> {
    item.problems
        .iter()
        .map(|problem| problem.target_id.as_deref())
        .collect()
}

fn ticket<'a>(list: &'a ItemListDto, id: &str) -> &'a ItemDto {
    list.items
        .iter()
        .find(|item| item.id.as_deref() == Some(id))
        .unwrap()
}

#[test]
fn lists_and_complete_reads_carry_relationships_and_keep_them_out_of_unknown_metadata() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let files = write_related_items(root);
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let tickets = all_tickets(&enabled.service, &repo);
    let documents = enabled.service.list_documents(&repo).unwrap();

    // Three tickets and no nonconforming entry: an ignored relationship
    // value does not make a file nonconforming.
    assert_eq!(tickets.items.len(), 3);
    assert_eq!(tickets.index.state, IndexState::Current);
    let a = ticket(&tickets, TICKET_A);
    assert_eq!(a.slug.as_deref(), Some(SLUG));
    assert_eq!(
        a.parent,
        Some(dependency(TICKET_B, DependencyState::Closed))
    );
    assert_eq!(
        a.deps,
        [
            dependency(TICKET_C, DependencyState::Open),
            dependency(TICKET_B, DependencyState::Closed),
            dependency(ABSENT, DependencyState::Unresolved),
        ]
    );
    // A waits for C, which waits for A, and for a ticket no context holds.
    let readiness = a.readiness.as_ref().unwrap();
    assert_eq!(readiness.state, ReadinessState::Blocked);
    assert_eq!(
        readiness
            .reasons
            .iter()
            .map(|reason| (reason.code, reason.ids.iter().map(String::as_str).collect()))
            .collect::<Vec<(ReadinessReasonCode, Vec<&str>)>>(),
        [
            (ReadinessReasonCode::OpenDependency, vec![TICKET_C]),
            (ReadinessReasonCode::UnresolvedDependency, vec![ABSENT]),
            (
                ReadinessReasonCode::DependencyCycle,
                vec![TICKET_A, TICKET_C]
            ),
        ]
    );
    assert_eq!(
        ticket(&tickets, TICKET_B).readiness.as_ref().unwrap().state,
        ReadinessState::Closed
    );
    assert_eq!(
        Value::Object(a.unknown_metadata.clone()),
        json!({"other": "kept"})
    );
    // Nothing in A's file was ignored. A and C wait for each other, which
    // each of them reports with the lowest ID of the two.
    assert_eq!(codes(a), [ProblemCode::DependencyCycle]);
    assert_eq!(target_ids(a), [Some(TICKET_A)]);
    let b = ticket(&tickets, TICKET_B);
    assert_eq!((&b.slug, &b.parent, b.deps.len()), (&None, &None, 0));
    let c = ticket(&tickets, TICKET_C);
    assert_eq!((&c.slug, &c.parent), (&None, &None));
    assert_eq!(c.deps, [dependency(TICKET_A, DependencyState::Open)]);
    assert!(c.unknown_metadata.is_empty());
    assert_eq!(
        codes(c),
        [
            ProblemCode::InvalidSlug,
            ProblemCode::RelationshipSelfReference,
            ProblemCode::DuplicateDependency,
            ProblemCode::RelationshipInvalidId,
            ProblemCode::RelationshipWrongType,
            ProblemCode::DependencyCycle,
        ]
    );
    // The ticket itself, the repeated dependency, nothing for a value that
    // was never an ID, and the lowest ID of the cycle.
    assert_eq!(
        target_ids(c),
        [
            None,
            Some(TICKET_C),
            Some(TICKET_A),
            None,
            None,
            Some(TICKET_A)
        ]
    );
    for problem in &c.problems {
        assert_eq!(problem.path.as_deref(), Some(c.path.as_str()));
    }
    // No text from the file is published with a problem.
    let serialized = serde_json::to_string(c).unwrap();
    assert!(!serialized.contains("Not-A-Slug") && !serialized.contains("not-an-id"));
    // On a document the keys are unknown metadata and nothing more.
    let document = &documents.items[0];
    assert_eq!(
        (&document.slug, &document.parent, document.deps.len()),
        (&None, &None, 0)
    );
    assert_eq!(
        Value::Object(document.unknown_metadata.clone()),
        json!({"deps": [TICKET_A], "slug": SLUG})
    );
    assert_eq!(codes(document), []);

    // A complete read gives the same fields from the file, and the index,
    // which stored the same, is current.
    for listed in &tickets.items {
        let shown = enabled
            .service
            .show_item(&repo, &item_id(listed.id.as_deref().unwrap()))
            .unwrap();
        assert_eq!(shown.index.state, IndexState::Current, "{listed:?}");
        assert_eq!(
            (&shown.slug, &shown.parent, &shown.deps, &shown.problems),
            (&listed.slug, &listed.parent, &listed.deps, &listed.problems)
        );
        assert_eq!(shown.unknown_metadata, listed.unknown_metadata);
        let by_path = enabled
            .service
            .show_path(&repo, None, Path::new(&listed.path))
            .unwrap();
        assert_eq!(by_path.index.state, IndexState::Current);
        assert_eq!(
            (&by_path.slug, &by_path.parent, &by_path.deps),
            (&listed.slug, &listed.parent, &listed.deps)
        );
    }
    // The file still holds the keys, as the source shows.
    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    assert!(shown.source.unwrap().contains(&format!("slug: {SLUG}\n")));

    // The short code filter matches the whole code without regard to case.
    let by_slug = |slug: &str| {
        let list = enabled
            .service
            .list_tickets(
                &repo,
                &TicketFilter {
                    slug: Some(slug.to_owned()),
                    ..Default::default()
                },
            )
            .unwrap();
        list.items
            .iter()
            .map(|item| item.id.clone().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(by_slug(SLUG), [TICKET_A]);
    assert_eq!(by_slug(&SLUG.to_uppercase()), [TICKET_A]);
    assert!(by_slug("mh-vh-k9x2").is_empty());
    assert!(by_slug("Not-A-Slug").is_empty());

    // The lists read no item file: with none left they are what they were.
    for file in &files {
        fs::remove_file(file).unwrap();
    }
    assert!(all_tickets(&enabled.service, &repo) == tickets);
    assert!(enabled.service.list_documents(&repo).unwrap() == documents);
    assert_git_transport_uninitialized();
}

#[test]
fn a_dependency_on_a_document_is_left_out_and_reported_when_read() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "D", ""));
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(
            TICKET_A,
            "A",
            &format!("parent: {DOCUMENT_A}\ndeps: [{DOCUMENT_A}, {TICKET_B}]\n"),
        ),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    // Whether a target is a ticket is not known from one file, so the
    // edges are stored and the index holds no problem for them.
    let stored = stored_relationships(enabled.data_directory.path());
    assert_eq!(stored.edges.len(), 3);
    assert!(stored.problems.is_empty());
    let listed = all_tickets(&enabled.service, &repo);
    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    for item in [&listed.items[0], &shown] {
        assert_eq!(item.parent, None);
        assert_eq!(
            item.deps,
            [dependency(TICKET_B, DependencyState::Unresolved)]
        );
        // The parent and the dependency name the same document: one problem.
        assert_eq!(codes(item), [ProblemCode::RelationshipNotATicket]);
        assert_eq!(target_ids(item), [Some(DOCUMENT_A)]);
        assert_eq!(item.index.state, IndexState::Current);
    }

    // What an edge resolves to is decided when it is read: the ticket it
    // waited for arrives, closed, and this ticket's file is not touched.
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "B", CLOSURE),
    );
    refresh_completely(&enabled.service, root);
    let listed = all_tickets(&enabled.service, &repo);
    assert_eq!(
        ticket(&listed, TICKET_A).deps,
        [dependency(TICKET_B, DependencyState::Closed)]
    );
    assert_git_transport_uninitialized();
}

#[test]
fn removing_a_dependency_and_refreshing_removes_its_edge() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let files = write_related_items(root);
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let data = enabled.data_directory.path();

    // A loses its parent, its short code and all but one dependency; C
    // loses every invalid value.
    fs::write(
        &files[0],
        ticket_source(TICKET_A, "A", &format!("deps:\n  - {TICKET_B}\n")),
    )
    .unwrap();
    fs::write(&files[2], ticket_source(TICKET_C, "C", "")).unwrap();

    // Until the index is refreshed a complete read gives the file's and
    // says the index is behind; the list gives what the index holds.
    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_eq!((&shown.slug, &shown.parent), (&None, &None));
    assert_eq!(shown.deps, [dependency(TICKET_B, DependencyState::Closed)]);
    let listed = all_tickets(&enabled.service, &repo);
    assert_eq!(ticket(&listed, TICKET_A).deps.len(), 3);

    refresh_completely(&enabled.service, root);

    let text = |value: &str| value.to_owned();
    let stored = stored_relationships(data);
    assert_eq!(
        stored.edges,
        [(text(TICKET_A), text(TICKET_B), text("deps"))]
    );
    assert!(stored.problems.is_empty());
    assert!(stored.slugs.iter().all(|(_, slug)| slug.is_none()));
    assert_eq!(orphans(data, "item_edges"), 0);
    assert_eq!(orphans(data, "item_problems"), 0);
    let listed = all_tickets(&enabled.service, &repo);
    let a = ticket(&listed, TICKET_A);
    assert_eq!(a.deps, [dependency(TICKET_B, DependencyState::Closed)]);
    assert_eq!((&a.slug, &a.parent), (&None, &None));
    assert_eq!(codes(ticket(&listed, TICKET_C)), []);

    // An item that is gone takes its edges with it.
    fs::remove_dir_all(files[0].parent().unwrap()).unwrap();
    refresh_completely(&enabled.service, root);
    assert!(stored_relationships(data).edges.is_empty());
    assert_eq!(orphans(data, "item_edges"), 0);
    assert_git_transport_uninitialized();
}

const RELATIONSHIP_OBJECTS: [(&str, &str); 4] = [
    ("index", "item_problems_item_id_idx"),
    ("index", "discovered_items_slug"),
    ("table", "item_problems"),
    ("table", "item_edges"),
];

fn schema_objects(connection: &Connection) -> Vec<(String, String)> {
    connection
        .prepare("SELECT type, name FROM sqlite_master")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Makes the index what a build from before relationships left: the same
/// rows, without the short code column, the edges and the item problems.
fn remove_relationship_schema(data_directory: &Path) {
    let connection = index(data_directory);
    for (kind, name) in RELATIONSHIP_OBJECTS {
        connection
            .execute_batch(&format!("DROP {kind} {name}"))
            .unwrap();
    }
    connection
        .execute_batch("ALTER TABLE discovered_items DROP COLUMN slug")
        .unwrap();
    assert!(!columns(&connection, "discovered_items").contains(&"slug".to_owned()));
    let objects = schema_objects(&connection);
    for (kind, name) in RELATIONSHIP_OBJECTS {
        assert!(!objects.contains(&(kind.to_owned(), name.to_owned())));
    }
}

#[test]
fn an_index_from_before_relationships_gains_them_is_marked_and_reads_as_stale() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    {
        let service = RepositoryService::open_at(data.path()).unwrap();
        for fixture in [&first, &second] {
            service
                .enable(support::enable_request(&fixture.root))
                .unwrap();
            write_related_items(&fixture.root);
            refresh_completely(&service, &fixture.root);
        }
    }
    assert_eq!(refresh_required(data.path()), [false, false]);
    let refreshed_before = refreshed_at(data.path());
    remove_relationship_schema(data.path());

    let service = RepositoryService::open_at(data.path()).unwrap();

    let connection = index(data.path());
    assert!(columns(&connection, "discovered_items").contains(&"slug".to_owned()));
    let objects = schema_objects(&connection);
    for (kind, name) in RELATIONSHIP_OBJECTS {
        assert!(
            objects.contains(&(kind.to_owned(), name.to_owned())),
            "{kind} {name}"
        );
    }
    drop(connection);
    // Every registration is marked, and what the index held is kept.
    assert_eq!(refresh_required(data.path()), [true, true]);
    assert_eq!(refreshed_at(data.path()), refreshed_before);
    let stored = stored_relationships(data.path());
    assert_eq!(stored.slugs.len(), 8);
    assert!(stored.slugs.iter().all(|(_, slug)| slug.is_none()));
    assert!(stored.edges.is_empty() && stored.problems.is_empty());

    // The index holds no dependency for any ticket, and says it is behind
    // instead of saying that no ticket has one.
    let repo = service.resolve_repository(&first.root).unwrap();
    let tickets = all_tickets(&service, &repo);
    assert_eq!(tickets.index.state, IndexState::Stale);
    assert_eq!(tickets.items.len(), 3);
    for item in &tickets.items {
        assert_eq!(item.index.state, IndexState::Stale);
        assert_eq!(
            (&item.slug, &item.parent, item.deps.len()),
            (&None, &None, 0)
        );
    }
    // A complete read uses the file, so it has what the index lacks.
    let shown = service.show_item(&repo, &item_id(TICKET_A)).unwrap();
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_eq!(shown.slug.as_deref(), Some(SLUG));
    assert_eq!(shown.deps.len(), 3);

    // Migrating again changes nothing: the schema is the same, and a
    // registration refreshed since is not marked a second time.
    refresh_completely(&service, &first.root);
    drop(service);
    // In root order, which is not the order they were registered in.
    let marked = refresh_required(data.path());
    assert_eq!(marked.iter().filter(|marked| **marked).count(), 1);
    let definitions = table_definitions(data.path());
    let service = RepositoryService::open_at(data.path()).unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(table_definitions(data.path()), definitions);
    assert_eq!(refresh_required(data.path()), marked);
    // The refreshed registration has its relationships again.
    let tickets = all_tickets(&service, &repo);
    assert_eq!(tickets.index.state, IndexState::Current);
    assert_eq!(ticket(&tickets, TICKET_A).slug.as_deref(), Some(SLUG));
    assert_eq!(ticket(&tickets, TICKET_A).deps.len(), 3);
    assert_git_transport_uninitialized();
}

#[test]
fn processes_that_open_an_index_from_before_relationships_together_all_succeed() {
    let (fixture, enabled) = enabled();
    write_related_items(&fixture.root);
    refresh_completely(&enabled.service, &fixture.root);
    let support::EnabledRepository {
        service,
        data_directory,
    } = enabled;
    drop(service);
    remove_relationship_schema(data_directory.path());

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let opens: Vec<_> = (0..4)
        .map(|_| {
            let (path, barrier) = (data_directory.path().to_owned(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                RepositoryService::open_at(&path)
                    .map(drop)
                    .map_err(|error| error.kind)
            })
        })
        .collect();
    for open in opens {
        open.join().unwrap().unwrap();
    }

    let connection = index(data_directory.path());
    let objects = schema_objects(&connection);
    for (kind, name) in RELATIONSHIP_OBJECTS {
        assert_eq!(
            objects
                .iter()
                .filter(|object| **object == (kind.to_owned(), name.to_owned()))
                .count(),
            1,
            "{kind} {name}"
        );
    }
    assert_eq!(
        columns(&connection, "discovered_items")
            .iter()
            .filter(|column| column.as_str() == "slug")
            .count(),
        1
    );
    drop(connection);
    assert_eq!(refresh_required(data_directory.path()), [true]);
    assert_git_transport_uninitialized();
}

/// Creates the item worktree for a new ticket and saves it there.
fn create_ticket_context(service: &RepositoryService, root: &Path, id: &str) -> PathBuf {
    // As for a document: enabling leaves the configuration looking changed.
    let repository = git2::Repository::open(root).unwrap();
    let mut git_index = repository.index().unwrap();
    git_index.read(true).unwrap();
    git_index
        .add_path(Path::new(".manyhands/config.toml"))
        .unwrap();
    git_index.write().unwrap();
    save_ticket(service, root, id, ContextIntent::Create, "Created");
    context_worktree(root, id).join(ticket_path(id))
}

/// Saves the ticket through the ordinary save, with the file as it is now
/// as the expected observation.
fn save_ticket(
    service: &RepositoryService,
    root: &Path,
    id: &str,
    intent: ContextIntent,
    title: &str,
) {
    let file = root
        .join(".manyhands/worktrees")
        .join(id)
        .join(ticket_path(id));
    let outcome = service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: root.to_owned(),
                kind: AuthoringKind::Ticket,
                item_id: item_id(id),
                intent,
                operation_id: support::new_operation_id(),
            },
            draft: TicketDraft {
                title: title.to_owned(),
                ticket_type: "task".to_owned(),
                status: "open".to_owned(),
                project: None,
                team: None,
                body: format!("Body of {title}.\n"),
            },
            expected_path: match fs::read(&file) {
                Ok(bytes) => ExpectedPathObservation::from_bytes(&bytes),
                Err(_) => ExpectedPathObservation::Missing,
            },
        })
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Saved { .. }));
}

fn relationships_in(file: &Path, id: &str) -> TicketRelationships {
    let source = fs::read_to_string(file).unwrap();
    match parse_item(Path::new(&ticket_path(id)), &source).unwrap() {
        CanonicalItem::Ticket(ticket) => ticket_relationships(&ticket),
        other => panic!("not a ticket: {other:?}"),
    }
}

/// The front matter lines of the relationship keys, with the list entries
/// under `deps`.
fn relationship_lines(file: &Path) -> Vec<String> {
    fs::read_to_string(file)
        .unwrap()
        .lines()
        .filter(|line| {
            ["slug:", "parent:", "deps:", "- "]
                .iter()
                .any(|start| line.starts_with(start))
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_existing_ticket_save_keeps_hand_written_relationship_fields() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = create_ticket_context(&enabled.service, root, TICKET_A);
    // Written by hand, in flow form, between the fields the save writes.
    let created = fs::read_to_string(&file).unwrap();
    let by_hand = created.replacen(
        "---\n",
        &format!(
            "---\nslug: {SLUG}\nparent: {TICKET_B}\ndeps: [{TICKET_C}, \"{TICKET_B}\"]\nother: kept\n"
        ),
        1,
    );
    assert_ne!(by_hand, created);
    fs::write(&file, &by_hand).unwrap();
    let before = relationships_in(&file, TICKET_A);
    assert_eq!(before.slug.as_deref(), Some(SLUG));
    assert_eq!(before.parent, Some(item_id(TICKET_B)));
    assert_eq!(before.deps, [item_id(TICKET_C), item_id(TICKET_B)]);
    assert!(before.problems.is_empty());

    save_ticket(
        &enabled.service,
        root,
        TICKET_A,
        ContextIntent::Edit,
        "Renamed",
    );

    let saved = fs::read_to_string(&file).unwrap();
    assert!(saved.contains("title: Renamed\n"), "{saved}");
    assert!(saved.contains("other: kept\n"), "{saved}");
    // The same values, each key written once. The flow list became a block
    // list: the serializer keeps values, not formatting.
    assert_eq!(relationships_in(&file, TICKET_A), before);
    let lines = relationship_lines(&file);
    assert_eq!(
        lines,
        [
            format!("slug: {SLUG}"),
            format!("parent: {TICKET_B}"),
            "deps:".to_owned(),
            format!("- {TICKET_C}"),
            format!("- {TICKET_B}"),
        ]
    );

    // Saved again from that form, the fields come back byte for byte.
    save_ticket(
        &enabled.service,
        root,
        TICKET_A,
        ContextIntent::Edit,
        "Renamed again",
    );
    assert_eq!(relationship_lines(&file), lines);
    assert_eq!(relationships_in(&file, TICKET_A), before);

    // And the index, which each save refreshed, read them from that file.
    let repo = enabled.service.resolve_repository(root).unwrap();
    let listed = all_tickets(&enabled.service, &repo);
    let a = ticket(&listed, TICKET_A);
    assert_eq!(a.context.kind, ItemContextKind::Active);
    assert_eq!(a.slug.as_deref(), Some(SLUG));
    assert_eq!(
        a.parent,
        Some(dependency(TICKET_B, DependencyState::Unresolved))
    );
    assert_eq!(
        a.deps,
        [
            dependency(TICKET_C, DependencyState::Unresolved),
            dependency(TICKET_B, DependencyState::Unresolved),
        ]
    );
    assert_eq!(
        Value::Object(a.unknown_metadata.clone()),
        json!({"other": "kept"})
    );
    assert_git_transport_uninitialized();
}

#[test]
fn edges_come_only_from_the_effective_copy_of_each_ticket() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let in_primary = write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(
            TICKET_B,
            "B",
            &format!("slug: {SLUG}\ndeps: [{TICKET_C}]\n"),
        ),
    );
    commit(&fixture, &[&ticket_path(TICKET_B)], 1_000);
    let own = create_ticket_context(&enabled.service, root, TICKET_A);
    let worktree = context_worktree(root, TICKET_A);
    // A's worktree holds B as part of its checkout. That copy is edited
    // there, where it is not B's effective copy.
    let copy = worktree.join(ticket_path(TICKET_B));
    assert!(copy.is_file());
    fs::write(
        &copy,
        ticket_source(
            TICKET_B,
            "B",
            &format!("slug: zz-zz-zzzzz\ndeps: [{ABSENT}]\n"),
        ),
    )
    .unwrap();
    // A's own copy, in its worktree, is its effective one.
    fs::write(
        &own,
        ticket_source(
            TICKET_A,
            "A",
            &format!("parent: {TICKET_B}\ndeps: [{TICKET_B}]\n"),
        ),
    )
    .unwrap();

    refresh_completely(&enabled.service, root);

    let text = |value: &str| value.to_owned();
    let expected = StoredRelationships {
        slugs: vec![(text(TICKET_A), None), (text(TICKET_B), Some(text(SLUG)))],
        edges: vec![
            (text(TICKET_A), text(TICKET_B), text("parent")),
            (text(TICKET_A), text(TICKET_B), text("deps")),
            (text(TICKET_B), text(TICKET_C), text("deps")),
        ],
        problems: Vec::new(),
    };
    assert_eq!(
        stored_relationships(enabled.data_directory.path()),
        expected
    );
    let repo = enabled.service.resolve_repository(root).unwrap();
    let listed = all_tickets(&enabled.service, &repo);
    let a = ticket(&listed, TICKET_A);
    assert_eq!(a.context.kind, ItemContextKind::Active);
    assert_eq!(a.deps, [dependency(TICKET_B, DependencyState::Open)]);
    let b = ticket(&listed, TICKET_B);
    assert_eq!(b.context.kind, ItemContextKind::Primary);
    assert_eq!(b.deps, [dependency(TICKET_C, DependencyState::Unresolved)]);

    // A rebuild chooses the same copies.
    rebuild(&enabled.service, root);
    assert_eq!(
        stored_relationships(enabled.data_directory.path()),
        expected
    );
    // When B's own copy changes, its edges follow on the next refresh.
    fs::write(&in_primary, ticket_source(TICKET_B, "B", "")).unwrap();
    refresh_completely(&enabled.service, root);
    let stored = stored_relationships(enabled.data_directory.path());
    assert_eq!(stored.edges.len(), 2);
    assert!(stored.slugs.iter().all(|(_, slug)| slug.is_none()));
    assert_git_transport_uninitialized();
}

#[test]
fn a_stored_relationship_row_that_discovery_could_not_have_written_fails_the_read() {
    let read = |change: &str| {
        let (fixture, enabled) = enabled();
        write_related_items(&fixture.root);
        refresh_completely(&enabled.service, &fixture.root);
        let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
        index(enabled.data_directory.path())
            .execute_batch(change)
            .unwrap();
        let listed = enabled
            .service
            .list_tickets(&repo, &TicketFilter::default());
        let shown = enabled.service.show_item(&repo, &item_id(TICKET_B));
        assert_git_transport_uninitialized();
        (listed, shown.map(|_| ()))
    };
    let item = |id: &str| format!("(SELECT id FROM discovered_items WHERE item_id = '{id}')");

    assert!(read("").0.is_ok());
    for change in [
        // A short code the grammar does not allow, and one on a document.
        format!("UPDATE discovered_items SET slug = 'Free text' WHERE item_id = '{TICKET_B}'"),
        format!("UPDATE discovered_items SET slug = '{SLUG}' WHERE item_id = '{DOCUMENT_A}'"),
        // A target that is not an item ID, and one that is the ticket.
        format!(
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES ({}, 'free text', 'deps')",
            item(TICKET_B)
        ),
        format!(
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES ({}, '{TICKET_B}', 'deps')",
            item(TICKET_B)
        ),
        // A second parent, and an edge of a document.
        format!(
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES ({}, '{TICKET_C}', 'parent')",
            item(TICKET_A)
        ),
        format!(
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES ({}, '{TICKET_C}', 'deps')",
            item(DOCUMENT_A)
        ),
        // A detail that is not an item ID, and a problem of a document.
        format!(
            "INSERT INTO item_problems (item_id, code, detail) VALUES ({}, 'invalid-slug', 'free text')",
            item(TICKET_B)
        ),
        format!(
            "INSERT INTO item_problems (item_id, code) VALUES ({}, 'invalid-slug')",
            item(DOCUMENT_A)
        ),
    ] {
        let (listed, shown) = read(&change);
        assert_eq!(
            listed.unwrap_err().code(),
            ResultCode::InternalError,
            "{change}"
        );
        assert_eq!(
            shown.unwrap_err().code(),
            ResultCode::InternalError,
            "{change}"
        );
    }

    // A code this build does not know as a relationship problem is not
    // published: neither it nor a file problem's code stored here.
    for code in ["future-code", "Free text.", "invalid-path"] {
        let (listed, _) = read(&format!(
            "INSERT INTO item_problems (item_id, code) VALUES ({}, '{code}')",
            item(TICKET_B)
        ));
        let listed = listed.unwrap();
        assert_eq!(
            codes(ticket(&listed, TICKET_B)),
            [ProblemCode::UnknownProblem]
        );
        assert!(!serde_json::to_string(&listed).unwrap().contains(code));
    }
}

use support::items::{COMMENT_A, write_comment};

#[test]
fn a_dependency_or_parent_that_names_a_comment_names_no_ticket() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "B", ""),
    );
    write_comment(root, TICKET_B, COMMENT_A, None, "2026-09-30T12:00:00Z", "");
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "D", ""));
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(
            TICKET_A,
            "A",
            &format!("parent: {COMMENT_A}\ndeps: [{DOCUMENT_A}, {COMMENT_A}, {TICKET_B}]\n"),
        ),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let listed = all_tickets(&enabled.service, &repo);
    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();

    for item in [ticket(&listed, TICKET_A), &shown] {
        // A comment is in the index, so the ID is not one that a fetch
        // could still bring a ticket for.
        assert!(item.parent.is_none(), "{item:?}");
        assert_eq!(item.deps, [dependency(TICKET_B, DependencyState::Open)]);
        assert_eq!(
            codes(item),
            [
                ProblemCode::RelationshipNotATicket,
                ProblemCode::RelationshipNotATicket
            ]
        );
        // The comment once, as parent and as dependency, then the document.
        assert_eq!(target_ids(item), [Some(COMMENT_A), Some(DOCUMENT_A)]);
        assert_eq!(item.index.state, IndexState::Current);
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_short_code_written_in_uppercase_is_stored_and_read_in_lowercase() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let file = write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "A", "slug: MH-VH-K9X2B\n"),
    );
    let written = fs::read(&file).unwrap();
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    assert_eq!(
        stored_relationships(enabled.data_directory.path()).slugs,
        [(TICKET_A.to_owned(), Some(SLUG.to_owned()))]
    );
    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    for slug in [SLUG, "MH-VH-K9X2B", "mH-Vh-K9x2B"] {
        let listed = enabled
            .service
            .list_tickets(
                &repo,
                &TicketFilter {
                    slug: Some(slug.to_owned()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(listed.items.len(), 1, "{slug}");
        for item in [&listed.items[0], &shown] {
            assert_eq!(item.slug.as_deref(), Some(SLUG));
            assert_eq!(codes(item), []);
            assert_eq!(item.index.state, IndexState::Current);
        }
    }
    // The file is not rewritten, and the source shows it as written.
    assert_eq!(fs::read(&file).unwrap(), written);
    assert!(shown.source.unwrap().contains("slug: MH-VH-K9X2B\n"));
    assert_git_transport_uninitialized();
}

#[test]
fn relationship_keys_an_older_index_stored_as_unknown_metadata_are_not_repeated() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write_related_items(root);
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // What a build from before the keys were fields stored for a ticket
    // and, as this build still does, for a document.
    let older = format!(
        r#"{{"not_representable":false,"values":{{"deps":["{TICKET_C}"],"other":"kept","parent":"{TICKET_B}","slug":"{SLUG}"}}}}"#
    );
    index(enabled.data_directory.path())
        .execute(
            "UPDATE discovered_items SET unknown_metadata = ?1 WHERE item_id IN (?2, ?3)",
            [older.as_str(), TICKET_A, DOCUMENT_A],
        )
        .unwrap();

    let tickets = all_tickets(&enabled.service, &repo);
    let documents = enabled.service.list_documents(&repo).unwrap();

    let a = ticket(&tickets, TICKET_A);
    assert_eq!(
        Value::Object(a.unknown_metadata.clone()),
        json!({"other": "kept"})
    );
    assert_eq!(a.slug.as_deref(), Some(SLUG));
    assert_eq!(documents.items[0].unknown_metadata.len(), 4);
    assert_git_transport_uninitialized();
}

/// Adds a context row for the worktree of `worktree_id` holding a second
/// row for ticket A, with a short code and one dependency, on `target`.
fn insert_second_ticket_row(data_directory: &Path, root: &str, worktree_id: &str, target: &str) {
    let connection = index(data_directory);
    let repository_id: i64 = connection
        .query_row("SELECT id FROM repositories", [], |row| row.get(0))
        .unwrap();
    connection
        .execute(
            "INSERT INTO contexts
                (repository_id, kind, branch, worktree_path, item_id)
             VALUES (?1, 'active', ?2, ?3, ?4)",
            rusqlite::params![
                repository_id,
                format!("manyhands/ticket/{worktree_id}"),
                Path::new(root)
                    .join(".manyhands/worktrees")
                    .join(worktree_id)
                    .to_str()
                    .unwrap(),
                worktree_id
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO discovered_items
                (context_id, item_id, kind, canonical_path, title, ticket_type, status,
                 activity_at, activity_source, slug)
             VALUES (?1, ?2, 'ticket', ?3, 'Second row', 'task', 'open', 5, 'git', 'zz-zz-zzzzz')",
            rusqlite::params![
                connection.last_insert_rowid(),
                TICKET_A,
                ticket_path(TICKET_A)
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO item_edges (item_id, target_id, kind) VALUES (?1, ?2, 'deps')",
            rusqlite::params![connection.last_insert_rowid(), target],
        )
        .unwrap();
}

#[test]
fn a_ticket_the_index_holds_twice_shows_only_its_effective_rows_relationships() {
    let listed_a = |worktree_id: &str, with_directory: bool| {
        let (fixture, enabled) = enabled();
        let root = &fixture.root;
        write(
            root,
            &ticket_path(TICKET_A),
            &ticket_source(
                TICKET_A,
                "A",
                &format!("slug: {SLUG}\ndeps: [{TICKET_B}]\n"),
            ),
        );
        refresh_completely(&enabled.service, root);
        if with_directory {
            fs::create_dir_all(root.join(".manyhands/worktrees").join(worktree_id)).unwrap();
        }
        insert_second_ticket_row(
            enabled.data_directory.path(),
            &root_string(&fixture),
            worktree_id,
            ABSENT,
        );
        let repo = enabled.service.resolve_repository(root).unwrap();
        let list = all_tickets(&enabled.service, &repo);
        // One entry, and an index that holds an item twice is behind.
        assert_eq!(ids(&list), [Some(TICKET_A)]);
        assert_eq!(list.index.state, IndexState::Stale);
        let a = list.items[0].clone();
        assert_git_transport_uninitialized();
        (a.context.kind, a.slug, a.deps)
    };
    let primary = (
        ItemContextKind::Primary,
        Some(SLUG.to_owned()),
        vec![dependency(TICKET_B, DependencyState::Unresolved)],
    );
    let own_worktree = (
        ItemContextKind::Active,
        Some("zz-zz-zzzzz".to_owned()),
        vec![dependency(ABSENT, DependencyState::Unresolved)],
    );

    // A copy in another item's worktree is never the effective one.
    assert_eq!(listed_a(TICKET_C, true), primary);
    // The ticket's own worktree is, while it is there.
    assert_eq!(listed_a(TICKET_A, true), own_worktree);
    assert_eq!(listed_a(TICKET_A, false), primary);
}

/// How many sequences `value` is nested in, and what the innermost holds.
fn nesting(mut value: &Value) -> (usize, &Value) {
    let mut depth = 0;
    while let Value::Array(values) = value {
        depth += 1;
        value = &values[0];
    }
    (depth, value)
}

/// The deepest container of unknown metadata that is kept.
const KEPT_DEPTH: usize = 64;

#[test]
fn unknown_metadata_nested_past_a_fixed_depth_is_null_and_always_reads_back() {
    for depth in [63, 64, 65, 126, 127] {
        let (fixture, enabled) = enabled();
        let root = &fixture.root;
        let path = ticket_path(TICKET_A);
        write(
            root,
            &path,
            &ticket_source(
                TICKET_A,
                "Deep",
                &format!("deep: {}1{}\n", "[".repeat(depth), "]".repeat(depth)),
            ),
        );
        write(
            root,
            &ticket_path(TICKET_B),
            &ticket_source(TICKET_B, "Plain", "extra: kept\n"),
        );
        refresh_completely(&enabled.service, root);
        let repo = enabled.service.resolve_repository(root).unwrap();

        // One deep key fails no read of the registration.
        let list = enabled
            .service
            .list_tickets(&repo, &TicketFilter::default())
            .unwrap_or_else(|error| panic!("depth {depth}: {error:?}"));
        assert!(
            list.items
                .iter()
                .any(|item| item.id.as_deref() == Some(TICKET_B)),
            "depth {depth}"
        );
        enabled.service.list_documents(&repo).unwrap();
        enabled
            .service
            .show_item(&repo, &item_id(TICKET_B))
            .unwrap();
        let listed = list
            .items
            .iter()
            .find(|item| item.path == path)
            .unwrap_or_else(|| panic!("depth {depth}"));
        if listed.id.is_none() {
            // The front matter is nested further than YAML itself is read.
            assert_eq!(codes(listed), [ProblemCode::MalformedFrontMatter]);
            assert!(depth > 126, "depth {depth}");
            continue;
        }
        let shown = enabled
            .service
            .show_item(&repo, &item_id(TICKET_A))
            .unwrap_or_else(|error| panic!("depth {depth}: {error:?}"));

        for item in [listed, &shown] {
            let (kept, innermost) = nesting(&item.unknown_metadata["deep"]);
            if depth <= KEPT_DEPTH {
                assert_eq!((kept, innermost), (depth, &json!(1)), "depth {depth}");
                assert!(item.problems.is_empty(), "depth {depth}");
            } else {
                assert_eq!(
                    (kept, innermost),
                    (KEPT_DEPTH, &Value::Null),
                    "depth {depth}"
                );
                assert_eq!(
                    codes(item),
                    [ProblemCode::MetadataNotRepresentable],
                    "depth {depth}"
                );
            }
        }
        assert_eq!(shown.index.state, IndexState::Current, "depth {depth}");
    }
    assert_git_transport_uninitialized();
}

#[test]
fn stored_unknown_metadata_that_is_not_what_a_refresh_writes_is_an_invalid_row() {
    let deeper_than_json_reads = format!(
        r#"{{"not_representable":false,"values":{{"deep":{}1{}}}}}"#,
        "[".repeat(200),
        "]".repeat(200)
    );
    for stored in [
        "not json",
        r#"{"values":{}}"#,
        r#"["not_representable","values"]"#,
        deeper_than_json_reads.as_str(),
    ] {
        let (fixture, enabled) = enabled();
        let root = &fixture.root;
        write(
            root,
            &ticket_path(TICKET_A),
            &ticket_source(TICKET_A, "Tampered", "extra: kept\n"),
        );
        write(
            root,
            &ticket_path(TICKET_B),
            &ticket_source(TICKET_B, "Plain", "extra: kept\n"),
        );
        refresh_completely(&enabled.service, root);
        let repo = enabled.service.resolve_repository(root).unwrap();
        index(enabled.data_directory.path())
            .execute(
                "UPDATE discovered_items SET unknown_metadata = ?1 WHERE item_id = ?2",
                [stored, TICKET_A],
            )
            .unwrap();

        // No refresh writes this, so the row is not one a read answers
        // from, and the file is not blamed for it.
        let codes = [
            enabled
                .service
                .list_tickets(&repo, &TicketFilter::default())
                .unwrap_err()
                .code(),
            enabled
                .service
                .show_item(&repo, &item_id(TICKET_A))
                .unwrap_err()
                .code(),
            enabled
                .service
                .show_item(&repo, &item_id(TICKET_B))
                .unwrap_err()
                .code(),
            enabled.service.ticket_cycles(&repo).unwrap_err().code(),
        ];
        assert_eq!(codes, [ResultCode::InternalError; 4], "{stored}");
    }
    assert_git_transport_uninitialized();
}

// A complete read decides what the file names now, so it has to find a
// comment that no edge in the index names yet.
#[test]
fn a_complete_read_finds_a_comment_only_the_file_names() {
    use support::items::COMMENT_B;

    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(
        root,
        &ticket_path(TICKET_B),
        &ticket_source(TICKET_B, "B", ""),
    );
    write_comment(root, TICKET_B, COMMENT_A, None, "2026-09-30T12:00:00Z", "");
    write_comment(root, TICKET_B, COMMENT_B, None, "2026-09-30T12:00:01Z", "");
    let path = ticket_path(TICKET_A);
    let file = write(
        root,
        &path,
        &ticket_source(TICKET_A, "A", &format!("deps: [{COMMENT_A}]\n")),
    );
    // C has an edge to a comment in the index, and is not the ticket read.
    write(
        root,
        &ticket_path(TICKET_C),
        &ticket_source(TICKET_C, "C", &format!("deps: [{COMMENT_A}, {TICKET_A}]\n")),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // The file now names the other comment, a ticket, and an ID nothing has.
    fs::write(
        &file,
        ticket_source(
            TICKET_A,
            "A",
            &format!("parent: {COMMENT_B}\ndeps: [{COMMENT_B}, {TICKET_B}, {TICKET_D}]\n"),
        ),
    )
    .unwrap();

    let shown = enabled
        .service
        .show_item(&repo, &item_id(TICKET_A))
        .unwrap();
    let by_path = enabled
        .service
        .show_path(&repo, None, Path::new(&path))
        .unwrap();

    for item in [&shown, &by_path] {
        assert!(item.parent.is_none(), "{item:?}");
        assert_eq!(
            item.deps,
            [
                dependency(TICKET_B, DependencyState::Open),
                dependency(TICKET_D, DependencyState::Unresolved),
            ]
        );
        assert_eq!(codes(item), [ProblemCode::RelationshipNotATicket]);
        assert_eq!(target_ids(item), [Some(COMMENT_B)]);
        assert_eq!(item.index.state, IndexState::Stale);
    }
    // The list is the index's, which still names the first comment.
    let listed = all_tickets(&enabled.service, &repo);
    assert_eq!(target_ids(ticket(&listed, TICKET_A)), [Some(COMMENT_A)]);
    let other = enabled
        .service
        .show_item(&repo, &item_id(TICKET_C))
        .unwrap();
    assert_eq!(target_ids(&other), [Some(COMMENT_A)]);
    assert_eq!(other.deps, [dependency(TICKET_A, DependencyState::Open)]);
    assert_git_transport_uninitialized();
}

/// Stores a problem of `code` at `path` in the primary context, as a
/// refresh would.
fn insert_problem(data_directory: &Path, path: &str, code: &str) {
    index(data_directory)
        .execute(
            "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
             SELECT repository_id, id, ?1, ?2, 'unused', 0 FROM contexts WHERE kind = 'primary'",
            [path, code],
        )
        .unwrap();
}

#[test]
fn a_list_is_not_complete_when_the_refresh_could_not_read_all_of_its_directory() {
    // `(stored path, documents complete, tickets complete)`
    let cases = [
        ("docs", false, true),
        ("docs/sub", false, true),
        ("docs/a/b.c", false, true),
        // A file that could not be read is one file, not a directory.
        ("docs/link.md", true, true),
        ("docs/sub/link.md", true, true),
        (".manyhands/tickets", true, false),
        (".manyhands", true, false),
        // One ticket's directory or file.
        (".manyhands/tickets/linked", true, true),
        (
            ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FC9/ticket.md",
            true,
            true,
        ),
        // Where item worktrees are: either kind's effective copy can be in
        // one the refresh never saw.
        (".manyhands/worktrees", false, false),
        // One worktree is one item's.
        (
            ".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FC9",
            true,
            true,
        ),
        (".manyhands/comments", true, true),
        ("", true, true),
        ("docsx", true, true),
        ("notes", true, true),
    ];
    for (path, documents_complete, tickets_complete) in cases {
        let (fixture, enabled) = enabled();
        let root = &fixture.root;
        write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
        write(
            root,
            &ticket_path(TICKET_A),
            &ticket_source(TICKET_A, "A", ""),
        );
        refresh_completely(&enabled.service, root);
        let repo = enabled.service.resolve_repository(root).unwrap();
        let lists = || {
            (
                enabled.service.list_documents(&repo).unwrap(),
                all_tickets(&enabled.service, &repo),
            )
        };
        let (documents, tickets) = lists();
        assert!(documents.complete && tickets.complete);

        // Any other problem there says nothing about what was listed.
        insert_problem(enabled.data_directory.path(), path, "context");
        let (documents, tickets) = lists();
        assert!(documents.complete && tickets.complete, "{path}");

        insert_problem(enabled.data_directory.path(), path, "source");
        let (documents, tickets) = lists();
        assert_eq!(documents.complete, documents_complete, "{path}");
        assert_eq!(tickets.complete, tickets_complete, "{path}");
        // What the index does hold is still listed, and the index is not
        // behind: a refresh would stop at the same place.
        assert_eq!(ids(&documents), [Some(DOCUMENT_A)], "{path}");
        assert_eq!(ids(&tickets), [Some(TICKET_A)], "{path}");
        assert_eq!(documents.index.state, IndexState::Current);
        assert_eq!(tickets.index.state, IndexState::Current);
    }
    assert_git_transport_uninitialized();
}

/// A relative path as the index stores it, with the platform's separator.
fn native_path(components: &[&str]) -> Option<String> {
    let path: PathBuf = components.iter().collect();
    Some(path.to_str().unwrap().to_owned())
}

/// The `(path, code)` of every stored problem.
fn stored_problem_rows(data_directory: &Path) -> Vec<(Option<String>, String)> {
    index(data_directory)
        .prepare("SELECT path, code FROM problems ORDER BY path, code")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

// The refresh reads at most 1,024 entries of the tree under `docs`, and of
// `.manyhands/tickets`. It stops there and stores where it stopped.
#[test]
fn a_refresh_that_stops_at_its_entry_limit_leaves_the_lists_incomplete() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "A", ""));
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source(TICKET_A, "A", ""),
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    assert!(enabled.service.list_documents(&repo).unwrap().complete);
    assert!(all_tickets(&enabled.service, &repo).complete);

    fs::create_dir_all(root.join("docs/sub")).unwrap();
    for entry in 0..1025 {
        fs::write(root.join(format!("docs/sub/{entry:04}.txt")), "").unwrap();
    }
    refresh_completely(&enabled.service, root);

    assert!(
        stored_problem_rows(enabled.data_directory.path())
            .contains(&(native_path(&["docs", "sub"]), "source".to_owned()))
    );
    let documents = enabled.service.list_documents(&repo).unwrap();
    assert!(!documents.complete);
    assert_eq!(documents.index.state, IndexState::Current);
    assert!(all_tickets(&enabled.service, &repo).complete);

    fs::remove_dir_all(root.join("docs/sub")).unwrap();
    for entry in 0..1025 {
        fs::create_dir(root.join(format!(".manyhands/tickets/{entry:04}"))).unwrap();
    }
    refresh_completely(&enabled.service, root);

    assert_eq!(
        stored_problem_rows(enabled.data_directory.path())
            .into_iter()
            .filter(|(_, code)| code == "source")
            .collect::<Vec<_>>(),
        [(native_path(&[".manyhands", "tickets"]), "source".to_owned())]
    );
    let tickets = all_tickets(&enabled.service, &repo);
    assert!(!tickets.complete);
    assert_eq!(tickets.index.state, IndexState::Current);
    assert!(enabled.service.list_documents(&repo).unwrap().complete);
    assert_git_transport_uninitialized();
}
