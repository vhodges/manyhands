//! `Answer::naming`: what a `busy` or `recovery_required` answer names of
//! the operation in its way, by what its own read of that row finds.

use rusqlite::{Connection, params};

use super::*;
use crate::repository::{OperationDto, REGISTRY_FILE, mutation::dto::BlockerDto};

/// A registry with one repository, and a local row of `operation`, a ticket
/// save in `state`, whose `item_id` is `item`. An item that is not an ID
/// makes the row unreadable to an operation read, as a damaged row would.
fn registry_with_a_row(
    operation: OperationId,
    item: &str,
    state: &str,
) -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute(
        "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",
        [root.path().to_str().unwrap()],
    )
    .unwrap();
    db.execute(
        "INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,item_id,state,observed_at)
         VALUES(1,?1,?2,'save_ticket','t',?3,?4,1)",
        params![root.path().to_str().unwrap(), operation.to_string(), item, state],
    )
    .unwrap();
    (data, root, service)
}

/// A live blocker the wait named is named whatever its answer's own read
/// finds: in full when the row reads, finished since or not, by its ID and
/// family alone when it cannot be read. A row that is gone since names
/// nothing: the operation that was in the way is no more, and the same
/// request, which the answer offers, meets nothing of it. A blocker judged
/// stale, or not judged, whose row cannot be read names nothing, as before.
#[test]
fn an_answer_names_a_live_blocker_its_read_cannot_return() {
    let item = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
    let operation = OperationId::new();
    let elsewhere = OperationId::new();
    let minimal = |live: Option<bool>| BlockerDto {
        operation_id: operation.to_string(),
        family: OperationFamily::Local,
        action: None,
        item_id: None,
        request_id: None,
        author: None,
        host: None,
        pid: None,
        started_at: None,
        live,
    };
    // (case, item the row names, its state, blocker the domain names, its
    // judgment, the row as the wait read it, the answer's code before, its
    // code after, what it names)
    type Row<'a> = (
        &'a str,
        &'a str,
        &'a str,
        OperationId,
        Option<bool>,
        Option<&'a OperationDto>,
        ResultCode,
        ResultCode,
        Option<BlockerDto>,
    );
    let full = BlockerDto {
        action: Some(super::super::super::OperationAction::SaveTicket),
        item_id: Some(item.to_owned()),
        ..minimal(Some(true))
    };
    // The row as a wait that found it live read it, before it was damaged.
    let (_data, root, service) = registry_with_a_row(operation, item, "created");
    let crate::repository::BlockerRow::Found {
        operation: read, ..
    } = service.blocking_operation(
        root.path(),
        &Blocker {
            family: OperationFamily::Local,
            operation_id: operation,
            live: None,
            seen: None,
        },
    )
    else {
        panic!("the row reads");
    };
    let rows: [Row<'_>; 7] = [
        (
            "live, the row reads: named in full",
            item,
            "created",
            operation,
            Some(true),
            None,
            ResultCode::Busy,
            ResultCode::Busy,
            Some(full.clone()),
        ),
        (
            "live, the row finished since: named in full, busy still",
            item,
            "completed",
            operation,
            Some(true),
            None,
            ResultCode::Busy,
            ResultCode::Busy,
            Some(full.clone()),
        ),
        (
            "live, the row cannot be read: named by its ID and family",
            "not-an-item",
            "created",
            operation,
            Some(true),
            None,
            ResultCode::Busy,
            ResultCode::Busy,
            Some(minimal(Some(true))),
        ),
        (
            "live, the row cannot be read, the wait read it: named as the wait read it",
            "not-an-item",
            "created",
            operation,
            Some(true),
            Some(&read),
            ResultCode::Busy,
            ResultCode::Busy,
            Some(full.clone()),
        ),
        (
            "live, the row is gone: named nothing, busy still",
            item,
            "created",
            elsewhere,
            Some(true),
            None,
            ResultCode::Busy,
            ResultCode::Busy,
            None,
        ),
        (
            "stale, the row cannot be read: named nothing",
            "not-an-item",
            "created",
            operation,
            Some(false),
            None,
            ResultCode::RecoveryRequired,
            ResultCode::RecoveryRequired,
            None,
        ),
        (
            "not judged, the row cannot be read: named nothing",
            "not-an-item",
            "created",
            operation,
            None,
            None,
            ResultCode::Busy,
            ResultCode::Busy,
            None,
        ),
    ];
    for (case, item, state, named, live, seen, before, after, expected) in rows {
        let (_data, root, service) = registry_with_a_row(operation, item, state);
        let blocker = Blocker {
            family: OperationFamily::Local,
            operation_id: named,
            live,
            seen: seen.cloned().map(Box::new),
        };
        let answer = Answer::stopped(before).naming(&service, root.path(), Some(&blocker));
        assert_eq!(answer.code, after, "{case}");
        let expected = expected.map(|expected| BlockerDto {
            operation_id: named.to_string(),
            ..expected
        });
        assert_eq!(answer.blocker.map(|named| *named), expected, "{case}");
    }
}
