//! The scope of the hold: what an unfinished row of the local journal
//! refuses. A row of one item refuses that item and work on the whole
//! repository, not another item. A row an index owner may be at work
//! under, a row of the whole repository and a row whose item is not known
//! refuse everything. A refresh under a row's own ID takes that row up,
//! in that row's scope.

use super::*;

const ROOT: &str = "/repository";

const ITEM_X: &str = "01ARZ3NDEKTSV4RRFFQ69G5FXA";
const ITEM_Y: &str = "01ARZ3NDEKTSV4RRFFQ69G5FYA";
const ITEM_Z: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZA";

/// The IDs of the rows a case seeds, and of the call it makes.
const ROW_1: &str = "01ARZ3NDEKTSV4RRFFQ69G5F01";
const ROW_2: &str = "01ARZ3NDEKTSV4RRFFQ69G5F02";
const CALL: &str = "01ARZ3NDEKTSV4RRFFQ69G5F09";

/// A row as the journal holds it: its ID (`None` for a row from before
/// operations had IDs), action, target, the item it names, its state, and
/// the last step it recorded.
struct Seed {
    id: Option<&'static str>,
    action: &'static str,
    target: String,
    item_id: Option<&'static str>,
    state: &'static str,
    completed_step: Option<&'static str>,
}

/// What a call is.
#[derive(Clone, Copy)]
enum Call {
    /// A local operation of `operation`, under `id`, with `target`.
    Begin(RepositoryOperation, &'static str, Target),
    /// `Begin`, for a call that was checked against other items as the
    /// index holds them, as a ticket save that sets relationships is.
    Across(RepositoryOperation, &'static str, Target),
    /// The question a folder's creation asks before it makes anything:
    /// can a refresh under `id` begin?
    RefreshCanBegin(&'static str),
    /// The guard a poll asks before it reserves: `require_no_pending_local`
    /// (`remote/poll.rs`, and the reservation of any remote operation).
    Poll,
    /// The same guard, asked by the reservation of a synchronization of
    /// the item (`None`: of the primary).
    Sync(Option<&'static str>),
    /// The same guard, asked by the retirement of the item's context
    /// before it claims anything (`context_retirement/execution.rs`,
    /// `authority.rs`, `source.rs`).
    Retire(&'static str),
}

#[derive(Clone, Copy)]
enum Target {
    Ticket(&'static str),
    Document(&'static str),
    Release(&'static str),
    Prepare(&'static str),
    Repair(&'static str),
    /// The index hand-off of a synchronization of the item (P-4).
    SyncIndex(&'static str),
    /// The index hand-off of a synchronization of the primary (P-4).
    PrimaryIndex,
    Empty,
    Remote,
}

impl Target {
    fn text(self) -> String {
        match self {
            Self::Ticket(item) => ticket(item),
            Self::Document(item) => format!("authoring-context-v1/document/{item}/Edit\0docs/a.md"),
            Self::Release(item) => format!("authoring-context-v1/ticket/{item}/Release"),
            Self::Prepare(item) => format!("authoring-context-v1/ticket/{item}/Edit"),
            Self::Repair(item) => {
                format!(
                    "authoring-context-v1/ticket/{item}/Repair\0.manyhands/tickets/{item}/ticket.md"
                )
            }
            Self::SyncIndex(item) => format!("synchronization-index-v1/ticket/{item}/{OID}"),
            Self::PrimaryIndex => format!("synchronization-index-v1/primary/{OID}"),
            Self::Empty => String::new(),
            Self::Remote => "remote-v1-0123".to_owned(),
        }
    }
}

/// The commit a synchronization's hand-off row names.
const OID: &str = "0123456789abcdef0123456789abcdef01234567";

fn ticket(item: &str) -> String {
    format!("authoring-context-v1/ticket/{item}/Edit\0.manyhands/tickets/{item}/ticket.md")
}

/// A pending row of `action` on item `item`, which it names, in `state`.
fn item_row(
    id: &'static str,
    action: &'static str,
    item: &'static str,
    state: &'static str,
) -> Seed {
    let target = match action {
        "save_document" => Target::Document(item).text(),
        "release_context" => Target::Release(item).text(),
        "prepare_context" => Target::Prepare(item).text(),
        _ => ticket(item),
    };
    Seed {
        id: Some(id),
        action,
        target,
        item_id: Some(item),
        state,
        completed_step: None,
    }
}

/// A pending row of the whole repository.
fn repository_row(id: Option<&'static str>, action: &'static str, state: &'static str) -> Seed {
    Seed {
        id,
        action,
        target: String::new(),
        item_id: None,
        state,
        completed_step: None,
    }
}

/// How a call was answered.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// A new row was begun.
    Begun,
    /// The call's own pending row was taken up.
    TakenUp,
    /// Nothing was begun or taken up, and nothing stood in the way: the
    /// answer of a question that begins nothing.
    Clear,
    /// Refused `recovery_required`, naming the row in the way, or no row
    /// when the one in the way has no ID.
    Refused(Option<&'static str>),
    /// Refused `operation_mismatch`: the call's ID is that of a row that is
    /// not this call's, and nothing else is looked at.
    Mismatch,
}

/// The rows of the journal, as they stand: what a refused call must leave
/// as it found it.
fn rows(db: &Connection) -> Vec<(i64, String, String, String, Option<String>)> {
    db.prepare(
        "SELECT id, operation_ulid, action, state, item_id FROM operation_records ORDER BY id",
    )
    .unwrap()
    .query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn journal(seeds: &[Seed]) -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    crate::repository::discovery::migrate_registry(&mut db).unwrap();
    db.execute(
        "INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required)
         VALUES(1,?1,7,'accessible',0)",
        [ROOT],
    )
    .unwrap();
    for seed in seeds {
        db.execute(
            "INSERT INTO operation_records
                (repository_id, root_path, operation_ulid, action, target, item_id, state,
                 completed_step, observed_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, 7)",
            params![
                ROOT,
                seed.id,
                seed.action,
                seed.target,
                seed.item_id,
                seed.state,
                seed.completed_step
            ],
        )
        .unwrap();
    }
    db
}

fn answer(seeds: &[Seed], call: Call) -> Answer {
    let mut db = journal(seeds);
    let before = rows(&db);
    let root = Path::new(ROOT);
    let reach = match call {
        Call::Across(..) => CallReach::OtherItems,
        _ => CallReach::OwnItem,
    };
    let result = match call {
        Call::Begin(operation, id, target) | Call::Across(operation, id, target) => {
            begin_or_reconcile_operation_reaching(
                &mut db,
                root,
                operation,
                OperationId::parse(id).unwrap(),
                &target.text(),
                7,
                reach,
            )
            .map(|record| {
                if record.is_new {
                    Answer::Begun
                } else {
                    assert!(record.is_pending, "a call that neither began nor took up");
                    Answer::TakenUp
                }
            })
        }
        Call::RefreshCanBegin(id) => require_refresh_can_begin(
            &db,
            root,
            RepositoryOperation::CreateFolder,
            OperationId::parse(id).unwrap(),
        )
        .map(|()| Answer::Clear),
        // Today's guard is asked with no call: it refuses behind any
        // pending row. Task 14 passes it the call's scope (`CallScope`),
        // and these arms pass the item each call is of.
        Call::Poll | Call::Sync(_) | Call::Retire(_) => {
            // The item the call is of, which today's guard is not given.
            let _item: Option<&str> = match call {
                Call::Sync(item) => item,
                Call::Retire(item) => Some(item),
                _ => None,
            };
            require_no_pending_local(&db, 1).map(|()| Answer::Clear)
        }
    };
    match result {
        Ok(answer) => answer,
        Err(error) if error.kind == RepositoryErrorKind::OperationMismatch => {
            assert_eq!(rows(&db), before, "a refused call wrote");
            Answer::Mismatch
        }
        Err(error) => {
            assert_eq!(
                error.kind,
                RepositoryErrorKind::RecoveryRequired,
                "{error:?}"
            );
            assert_eq!(rows(&db), before, "a refused call wrote");
            let named = error
                .blocker()
                .map(|blocker| blocker.operation_id.to_string());
            Answer::Refused(match named.as_deref() {
                None => None,
                Some(ROW_1) => Some(ROW_1),
                Some(ROW_2) => Some(ROW_2),
                Some(other) => panic!("named an operation no row has: {other}"),
            })
        }
    }
}

use RepositoryOperation::{
    AddRemote, Enable, PrepareContext, RebuildRepository, RefreshRepository, ReleaseContext,
    RepairItem, SaveDocument, SaveTicket,
};

#[test]
fn an_unfinished_operation_blocks_its_own_item_and_repository_wide_work_but_not_other_items() {
    use Answer::{Begun, Clear, Mismatch, Refused, TakenUp};
    use Target::{Document, Empty, Prepare, Release, Remote, Repair, Ticket};

    let save_x_stopped = || {
        vec![item_row(
            ROW_1,
            "save_ticket",
            ITEM_X,
            "authoring_checkpoint_observed",
        )]
    };
    let save_x_owing = || vec![item_row(ROW_1, "save_ticket", ITEM_X, "failed")];
    let cases: Vec<(&str, Vec<Seed>, Call, Answer)> = vec![
        // A row of item X, stopped before its hand-off or owing it, that
        // no index owner is at work under.
        (
            "item row, same item",
            save_x_stopped(),
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_X)),
            Refused(Some(ROW_1)),
        ),
        (
            "item row, same item, another action",
            save_x_stopped(),
            Call::Begin(ReleaseContext, CALL, Release(ITEM_X)),
            Refused(Some(ROW_1)),
        ),
        (
            "item row, its own ID",
            save_x_stopped(),
            Call::Begin(SaveTicket, ROW_1, Ticket(ITEM_X)),
            TakenUp,
        ),
        // A call under a pending row's ID is a call for that row. For another
        // item the row is not in its way, so the ID check answers: the ID is
        // spent on another item, and nothing is written. (A scope that
        // judged the row in the way would answer `recovery_required`.)
        (
            "item row, its ID for another item",
            save_x_stopped(),
            Call::Begin(SaveTicket, ROW_1, Ticket(ITEM_Y)),
            Mismatch,
        ),
        (
            "item row, its ID for another item's document",
            save_x_stopped(),
            Call::Begin(SaveDocument, ROW_1, Document(ITEM_Y)),
            Mismatch,
        ),
        (
            "item row, another item",
            save_x_stopped(),
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "item row, another item's document",
            save_x_stopped(),
            Call::Begin(SaveDocument, CALL, Document(ITEM_Y)),
            Begun,
        ),
        (
            "item row, another item's context",
            save_x_stopped(),
            Call::Begin(PrepareContext, CALL, Prepare(ITEM_Y)),
            Begun,
        ),
        (
            "item row, another item's release",
            save_x_stopped(),
            Call::Begin(ReleaseContext, CALL, Release(ITEM_Y)),
            Begun,
        ),
        (
            "item row, another item's repair",
            save_x_stopped(),
            Call::Begin(RepairItem, CALL, Repair(ITEM_Y)),
            Begun,
        ),
        (
            "owing item row, another item",
            save_x_owing(),
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "retrying item row, another item",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "retry")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "handed-off item row, another item",
            vec![item_row(
                ROW_1,
                "save_ticket",
                ITEM_X,
                "authoritative_observed",
            )],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "item row, a refresh",
            save_x_stopped(),
            Call::Begin(RefreshRepository, CALL, Empty),
            Refused(Some(ROW_1)),
        ),
        (
            "item row, a rebuild",
            save_x_stopped(),
            Call::Begin(RebuildRepository, CALL, Empty),
            Refused(Some(ROW_1)),
        ),
        (
            "item row, a remote",
            save_x_stopped(),
            Call::Begin(AddRemote, CALL, Remote),
            Refused(Some(ROW_1)),
        ),
        (
            "item row, an enablement",
            save_x_stopped(),
            Call::Begin(Enable, CALL, Empty),
            Refused(Some(ROW_1)),
        ),
        (
            "item row, a folder's refresh",
            save_x_stopped(),
            Call::RefreshCanBegin(CALL),
            Refused(Some(ROW_1)),
        ),
        (
            "owing item row, its resume",
            save_x_owing(),
            Call::Begin(RefreshRepository, ROW_1, Empty),
            TakenUp,
        ),
        (
            "owing item row, a folder under its ID",
            save_x_owing(),
            Call::RefreshCanBegin(ROW_1),
            Clear,
        ),
        // Two items' rows, each owing its hand-off: each is resumed with the
        // other still pending, and a third item is written; the whole
        // repository waits for both.
        (
            "two item rows, the first's resume",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                item_row(ROW_2, "save_document", ITEM_Y, "retry"),
            ],
            Call::Begin(RefreshRepository, ROW_1, Empty),
            TakenUp,
        ),
        (
            "two item rows, the second's resume",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                item_row(ROW_2, "save_document", ITEM_Y, "retry"),
            ],
            Call::Begin(RefreshRepository, ROW_2, Empty),
            TakenUp,
        ),
        (
            "two item rows, a third item",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                item_row(ROW_2, "save_document", ITEM_Y, "retry"),
            ],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Z)),
            Begun,
        ),
        (
            "two item rows, the second item",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                item_row(ROW_2, "save_document", ITEM_Y, "retry"),
            ],
            Call::Begin(SaveDocument, CALL, Document(ITEM_Y)),
            Refused(Some(ROW_2)),
        ),
        (
            "two item rows, a refresh",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                item_row(ROW_2, "save_document", ITEM_Y, "retry"),
            ],
            Call::Begin(RefreshRepository, CALL, Empty),
            Refused(Some(ROW_1)),
        ),
        // The row in the way is the first that refuses: an item's row
        // that does not is passed over.
        (
            "an item row, then a refresh row",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                repository_row(Some(ROW_2), "refresh", "failed"),
            ],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_2)),
        ),
        // Rows of every authoring action are of their item.
        (
            "document row, another item",
            vec![item_row(ROW_1, "save_document", ITEM_X, "failed")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "document row, same item",
            vec![item_row(ROW_1, "save_document", ITEM_X, "failed")],
            Call::Begin(SaveDocument, CALL, Document(ITEM_X)),
            Refused(Some(ROW_1)),
        ),
        (
            "release row, another item",
            vec![item_row(
                ROW_1,
                "release_context",
                ITEM_X,
                "context_worktree_released",
            )],
            Call::Begin(ReleaseContext, CALL, Release(ITEM_Y)),
            Begun,
        ),
        (
            "preparation row, another item",
            vec![item_row(
                ROW_1,
                "prepare_context",
                ITEM_X,
                "branch_observed",
            )],
            Call::Begin(ReleaseContext, CALL, Release(ITEM_Y)),
            Begun,
        ),
        (
            "comment row, another item",
            vec![item_row(
                ROW_1,
                "submit_comment",
                ITEM_X,
                "authoring_checkpoint_observed",
            )],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "comment row, same item",
            vec![item_row(
                ROW_1,
                "submit_comment",
                ITEM_X,
                "authoring_checkpoint_observed",
            )],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_X)),
            Refused(Some(ROW_1)),
        ),
        // An index owner may be at work under the row with no lease held,
        // and nothing fences it from another row's claim: everything waits.
        (
            "indexing item row, another item",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "indexing")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "indexing item row, a refresh",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "indexing")],
            Call::Begin(RefreshRepository, CALL, Empty),
            Refused(Some(ROW_1)),
        ),
        (
            "indexing item row, its resume",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "indexing")],
            Call::Begin(RefreshRepository, ROW_1, Empty),
            TakenUp,
        ),
        (
            "invalidated item row, another item",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "index_invalidated")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "invalidated item row, a folder's refresh",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "index_invalidated")],
            Call::RefreshCanBegin(CALL),
            Refused(Some(ROW_1)),
        ),
        // A row that names no item, as every row of an older build but a
        // local synchronization's does, is of the whole repository.
        (
            "legacy item row, another item",
            vec![Seed {
                item_id: None,
                ..item_row(ROW_1, "save_ticket", ITEM_X, "failed")
            }],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "legacy item row, its own ID",
            vec![Seed {
                item_id: None,
                ..item_row(ROW_1, "save_ticket", ITEM_X, "failed")
            }],
            Call::Begin(SaveTicket, ROW_1, Ticket(ITEM_X)),
            TakenUp,
        ),
        (
            "legacy item row, its resume",
            vec![Seed {
                item_id: None,
                ..item_row(ROW_1, "save_ticket", ITEM_X, "failed")
            }],
            Call::Begin(RefreshRepository, ROW_1, Empty),
            TakenUp,
        ),
        // Rows of the whole repository refuse every other call.
        (
            "refresh row, an item",
            vec![repository_row(Some(ROW_1), "refresh", "failed")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "refresh row, its resume",
            vec![repository_row(Some(ROW_1), "refresh", "failed")],
            Call::Begin(RefreshRepository, ROW_1, Empty),
            TakenUp,
        ),
        (
            "rebuild row, an item",
            vec![repository_row(Some(ROW_1), "rebuild", "created")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "enablement row, an item",
            vec![repository_row(
                Some(ROW_1),
                "enable",
                "configuration_written",
            )],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "remote row, an item",
            vec![repository_row(Some(ROW_1), "add_remote", "remote_changed")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "registration row, an item",
            vec![repository_row(
                Some(ROW_1),
                "remove_registration",
                "created",
            )],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        // A local synchronization names its item, and rewrites the whole
        // index without a claim; a retirement removes a context and claims
        // the index for its request. Both are of the whole repository.
        (
            "local synchronization row, another item",
            vec![Seed {
                target: "observation".to_owned(),
                ..item_row(ROW_1, "context_sync_local", ITEM_X, "created")
            }],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "retirement row, another item",
            vec![Seed {
                item_id: None,
                target: format!("authoring-context-v1/ticket/{ITEM_X}/Retire"),
                ..item_row(ROW_1, "retire_context", ITEM_X, "created")
            }],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        // A row from before operations had IDs is a refresh's or a
        // rebuild's: a refresh takes it up, anything else waits for it.
        (
            "ID-less refresh row, a refresh",
            vec![repository_row(None, "refresh", "created")],
            Call::Begin(RefreshRepository, CALL, Empty),
            TakenUp,
        ),
        (
            "ID-less refresh row, an item",
            vec![repository_row(None, "refresh", "created")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(None),
        ),
        // A save that sets relationships was checked for cycles and kinds
        // against the other tickets the index holds, and an unfinished row
        // may hold a change the index has not been told of: every other
        // pending row is in its way, while a save that sets none is not.
        (
            "owing item row, another item's save setting relationships",
            save_x_owing(),
            Call::Across(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "stopped item row, another item's save setting relationships",
            save_x_stopped(),
            Call::Across(SaveTicket, CALL, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "owing item row, another item's plain save",
            save_x_owing(),
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        (
            "its own row and another item's, setting relationships",
            vec![
                item_row(ROW_1, "save_ticket", ITEM_X, "failed"),
                item_row(
                    ROW_2,
                    "save_ticket",
                    ITEM_Y,
                    "authoring_checkpoint_observed",
                ),
            ],
            Call::Across(SaveTicket, ROW_2, Ticket(ITEM_Y)),
            Refused(Some(ROW_1)),
        ),
        (
            "its own row alone, setting relationships",
            vec![item_row(
                ROW_2,
                "save_ticket",
                ITEM_Y,
                "authoring_checkpoint_observed",
            )],
            Call::Across(SaveTicket, ROW_2, Ticket(ITEM_Y)),
            TakenUp,
        ),
        (
            "no row, setting relationships",
            vec![],
            Call::Across(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
        ),
        // The row such a save leaves is of its own item: a call of
        // another item is not refused for it.
        (
            "a relationship save's own row, another item",
            vec![item_row(ROW_2, "save_ticket", ITEM_Y, "failed")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_X)),
            Begun,
        ),
    ];
    for (what, seeds, call, expected) in cases {
        assert_eq!(answer(&seeds, call), expected, "{what}");
    }
}

/// A row begun for an authoring action names its item, read from its
/// target, so the hold knows it; a row begun for any other action names
/// none.
#[test]
fn a_row_begun_for_an_authoring_action_names_its_item() {
    use Target::{Document, Empty, Prepare, Release, Remote, Repair, Ticket};
    for (operation, target, item) in [
        (SaveTicket, Ticket(ITEM_X), Some(ITEM_X)),
        (SaveDocument, Document(ITEM_X), Some(ITEM_X)),
        (PrepareContext, Prepare(ITEM_X), Some(ITEM_X)),
        (ReleaseContext, Release(ITEM_X), Some(ITEM_X)),
        (RepairItem, Repair(ITEM_X), Some(ITEM_X)),
        (RefreshRepository, Empty, None),
        (AddRemote, Remote, None),
    ] {
        let mut db = journal(&[]);
        begin_or_reconcile_operation(
            &mut db,
            Path::new(ROOT),
            operation,
            OperationId::parse(CALL).unwrap(),
            &target.text(),
            7,
        )
        .unwrap();
        let named: Option<String> = db
            .query_row("SELECT item_id FROM operation_records", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(named.as_deref(), item, "{operation:?}");
    }
}

// Table B of the fast-writes design (`docs/plans/2026-11-19-fast-writes-design.md`,
// "Table B: what a pending row holds"), written before the behaviour it
// pins and ignored until the design checkpoint (Task 3). A row holds a call
// when it is of the call's own item, or of the whole repository; or when the
// call is of the whole repository and the row does not owe only its
// hand-off; or when the call reached other items and the row is mid-write.
// Whether a held call waits (a live holder) or is refused at once (a stale
// one) is the lease's and the liveness judgement's, after these checks:
// here a held call is `Refused`, naming the row.

/// A save of `item` stopped before its commit was observed: mid-write.
fn mid_write(item: &'static str) -> Seed {
    Seed {
        completed_step: Some("authoring_destination_observed"),
        ..item_row(ROW_1, "save_ticket", item, "authoring_destination_observed")
    }
}

/// A save of `item` whose commit is observed and that owes only its
/// hand-off, in `state`.
fn owing(item: &'static str, state: &'static str) -> Seed {
    Seed {
        completed_step: Some("authoring_checkpoint_observed"),
        ..item_row(ROW_1, "save_ticket", item, state)
    }
}

/// The states of a row that owes only its hand-off, its commit observed:
/// a committed write whose state has not advanced yet (state and step
/// `authoring_checkpoint_observed`, which a crash between the checkpoint and
/// the hand-off leaves; today's `owes_only_its_hand_off` calls it mid-write,
/// the design's definition makes it owing, to be confirmed at the
/// checkpoint), the hand-off begun before indexing, a claim by an index
/// owner (live or crashed), a fence, and what a hand-off that did not bring
/// the index up to date leaves.
const OWING_STATES: [&str; 5] = [
    "authoring_checkpoint_observed",
    "authoritative_observed",
    "indexing",
    "index_invalidated",
    "failed",
];

/// The index hand-off row of a synchronization of `item` whose push landed
/// (P-4): a `refresh` row of the item, owing only its hand-off.
fn sync_hand_off(item: &'static str, state: &'static str) -> Seed {
    Seed {
        id: Some(ROW_1),
        action: "refresh",
        target: Target::SyncIndex(item).text(),
        item_id: Some(item),
        state,
        completed_step: Some("authoritative_observed"),
    }
}

/// The same row of a synchronization of the primary, which names no item
/// and is the primary's own (Owner question 2).
fn primary_hand_off(state: &'static str) -> Seed {
    Seed {
        item_id: None,
        target: Target::PrimaryIndex.text(),
        ..sync_hand_off(ITEM_X, state)
    }
}

/// The comment the registering row names.
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FCA";

/// A comment of `item` committed while its repository had no registration:
/// its local write and index step are done and its row is `completed`,
/// waiting for the registration. It owes only its hand-off.
fn registering_comment(item: &'static str) -> Seed {
    Seed {
        id: Some(ROW_1),
        action: "submit_comment",
        target: format!("authoring-context-v1/ticket/{item}/Edit\0.manyhands/comments\0{COMMENT}"),
        item_id: Some(item),
        state: "completed",
        completed_step: Some("comment_registration_pending"),
    }
}

/// The rows that keep their repository-wide hold (Table B's last two rows,
/// decisions 14 and 16): a running whole-repository refresh; a crashed
/// one; a rebuild; a crashed `context sync-local`; a retirement that owes
/// only its hand-off; and the plain whole-repository refresh row a close,
/// a promotion or a folder's creation indexes under. Each with whether a
/// refresh is refused naming it: one behind a local synchronization's or a
/// retirement's row is refused by that domain's own check first, which
/// names no row (as today).
fn repository_wide_rows() -> Vec<(&'static str, Seed, bool)> {
    vec![
        (
            "R5 a running whole-repository refresh",
            repository_row(Some(ROW_1), "refresh", "indexing"),
            true,
        ),
        (
            "R6 a crashed whole-repository refresh",
            repository_row(Some(ROW_1), "refresh", "failed"),
            true,
        ),
        (
            "R6 a rebuild",
            repository_row(Some(ROW_1), "rebuild", "retry"),
            true,
        ),
        (
            "R6 a crashed context sync-local",
            Seed {
                target: "observation".to_owned(),
                completed_step: Some("local_sync_prepared"),
                ..item_row(ROW_1, "context_sync_local", ITEM_X, "local_sync_prepared")
            },
            false,
        ),
        (
            "R6 a retirement owing its hand-off",
            Seed {
                item_id: None,
                target: format!("authoring-context-v1/ticket/{ITEM_X}/Retire"),
                completed_step: Some("context_retirement_absent"),
                ..item_row(ROW_1, "retire_context", ITEM_X, "authoritative_observed")
            },
            false,
        ),
        (
            "R6 a close's, promotion's or folder's index row",
            repository_row(Some(ROW_1), "refresh", "retry"),
            true,
        ),
    ]
}

#[test]
#[ignore = "fast writes: design checkpoint"]
fn table_b_what_a_pending_row_holds_at_the_local_begin_and_the_remote_guard() {
    use Answer::{Begun, Clear, Refused, TakenUp};
    use Target::{Empty, PrimaryIndex, SyncIndex, Ticket};

    let held = || Refused(Some(ROW_1));
    // The calls of Table B's columns, for a row of item X.
    let own_write = Call::Begin(SaveTicket, CALL, Ticket(ITEM_X));
    let other_write = Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y));
    let other_across = Call::Across(SaveTicket, CALL, Ticket(ITEM_Y));
    let refresh = Call::Begin(RefreshRepository, CALL, Empty);
    let rebuild = Call::Begin(RebuildRepository, CALL, Empty);
    let folder = Call::RefreshCanBegin(CALL);
    let resume = Call::Begin(RefreshRepository, ROW_1, Empty);

    let mut cases: Vec<(String, Vec<Seed>, Call, Answer)> = Vec::new();
    let mut cell = |row: &str, seed: &dyn Fn() -> Seed, column: &str, call: Call, expected| {
        cases.push((format!("{row} | {column}"), vec![seed()], call, expected));
    };

    // R1, R2: a per-item write mid-write. The lease tells live from stale.
    let row = "R1/R2 mid-write";
    let seed = || mid_write(ITEM_X);
    cell(row, &seed, "C1 a write on its item", own_write, held());
    cell(row, &seed, "C2 another item's write", other_write, Begun);
    cell(
        row,
        &seed,
        "C2 another item's write reaching other items",
        other_across,
        held(),
    );
    cell(
        row,
        &seed,
        "C2 another item's sync",
        Call::Sync(Some(ITEM_Y)),
        Clear,
    );
    cell(row, &seed, "C2 the primary's sync", Call::Sync(None), Clear);
    cell(
        row,
        &seed,
        "C1 its item's sync",
        Call::Sync(Some(ITEM_X)),
        held(),
    );
    cell(row, &seed, "C3 a whole refresh", refresh, held());
    cell(row, &seed, "C3 a rebuild", rebuild, held());
    cell(row, &seed, "C3 a folder's refresh", folder, held());
    cell(row, &seed, "C3 a poll", Call::Poll, held());
    cell(
        row,
        &seed,
        "C4 another item's retirement",
        Call::Retire(ITEM_Y),
        held(),
    );
    cell(
        row,
        &seed,
        "C4 its item's retirement",
        Call::Retire(ITEM_X),
        held(),
    );
    cell(
        row,
        &seed,
        "C5 its own retry",
        Call::Begin(SaveTicket, ROW_1, Ticket(ITEM_X)),
        TakenUp,
    );

    // R3: a per-item write that owes only its hand-off, live or crashed.
    for state in OWING_STATES {
        let row = format!("R3 owes only its hand-off ({state})");
        let row = row.as_str();
        let seed = || owing(ITEM_X, state);
        cell(row, &seed, "C1 a write on its item", own_write, held());
        cell(
            row,
            &seed,
            "C1 its item's sync",
            Call::Sync(Some(ITEM_X)),
            held(),
        );
        cell(row, &seed, "C2 another item's write", other_write, Begun);
        cell(
            row,
            &seed,
            "C2 another item's write reaching other items",
            other_across,
            Begun,
        );
        cell(
            row,
            &seed,
            "C2 another item's sync",
            Call::Sync(Some(ITEM_Y)),
            Clear,
        );
        cell(row, &seed, "C2 the primary's sync", Call::Sync(None), Clear);
        cell(row, &seed, "C3 a whole refresh", refresh, Begun);
        cell(row, &seed, "C3 a rebuild", rebuild, Begun);
        cell(row, &seed, "C3 a folder's refresh", folder, Clear);
        cell(row, &seed, "C3 a poll", Call::Poll, Clear);
        cell(
            row,
            &seed,
            "C4 another item's retirement",
            Call::Retire(ITEM_Y),
            Clear,
        );
        cell(
            row,
            &seed,
            "C4 its item's retirement",
            Call::Retire(ITEM_X),
            held(),
        );
        cell(row, &seed, "C5 its own resume", resume, TakenUp);
    }

    // R4: a sync whose only debt is its per-item index hand-off.
    for state in ["indexing", "failed"] {
        let row = format!("R4 a sync's hand-off ({state})");
        let row = row.as_str();
        let seed = || sync_hand_off(ITEM_X, state);
        cell(row, &seed, "C1 a write on its item", own_write, held());
        cell(
            row,
            &seed,
            "C1 its item's next sync",
            Call::Sync(Some(ITEM_X)),
            held(),
        );
        cell(row, &seed, "C2 another item's write", other_write, Begun);
        cell(
            row,
            &seed,
            "C2 another item's write reaching other items",
            other_across,
            Begun,
        );
        cell(
            row,
            &seed,
            "C2 another item's sync",
            Call::Sync(Some(ITEM_Y)),
            Clear,
        );
        cell(row, &seed, "C2 the primary's sync", Call::Sync(None), Clear);
        cell(row, &seed, "C3 a whole refresh", refresh, Begun);
        cell(row, &seed, "C3 a rebuild", rebuild, Begun);
        cell(row, &seed, "C3 a folder's refresh", folder, Clear);
        cell(row, &seed, "C3 a poll", Call::Poll, Clear);
        cell(
            row,
            &seed,
            "C4 another item's retirement",
            Call::Retire(ITEM_Y),
            Clear,
        );
        cell(
            row,
            &seed,
            "C4 its item's retirement",
            Call::Retire(ITEM_X),
            held(),
        );
        cell(
            row,
            &seed,
            "C5 its own sync's retry",
            Call::Begin(RefreshRepository, ROW_1, SyncIndex(ITEM_X)),
            TakenUp,
        );
    }

    // R4, Owner question 2: a primary sync's hand-off is the primary's own
    // item. A folder's creation writes the primary, as a close and a
    // promotion do.
    let row = "R4/Q2 a primary sync's hand-off";
    let seed = || primary_hand_off("indexing");
    cell(row, &seed, "C2 an item's write", own_write, Begun);
    cell(
        row,
        &seed,
        "C2 an item's write reaching other items",
        other_across,
        Begun,
    );
    cell(
        row,
        &seed,
        "C2 an item's sync",
        Call::Sync(Some(ITEM_X)),
        Clear,
    );
    cell(
        row,
        &seed,
        "C1 the primary's next sync",
        Call::Sync(None),
        held(),
    );
    cell(row, &seed, "C3 a whole refresh", refresh, Begun);
    cell(row, &seed, "C3 a rebuild", rebuild, Begun);
    cell(row, &seed, "C3 a poll", Call::Poll, Clear);
    cell(
        row,
        &seed,
        "C1 a folder's refresh, which writes the primary",
        folder,
        held(),
    );
    cell(
        row,
        &seed,
        "C4 an item's retirement",
        Call::Retire(ITEM_X),
        Clear,
    );
    cell(
        row,
        &seed,
        "C5 its own sync's retry",
        Call::Begin(RefreshRepository, ROW_1, PrimaryIndex),
        TakenUp,
    );

    // A comment in `comment_registration_pending` owes only its hand-off:
    // it holds its own item and nothing else.
    let row = "R3 a comment waiting for its registration";
    let seed = || registering_comment(ITEM_X);
    cell(row, &seed, "C1 a write on its item", own_write, held());
    cell(
        row,
        &seed,
        "C1 its item's sync",
        Call::Sync(Some(ITEM_X)),
        held(),
    );
    cell(row, &seed, "C2 another item's write", other_write, Begun);
    cell(
        row,
        &seed,
        "C2 another item's write reaching other items",
        other_across,
        Begun,
    );
    cell(
        row,
        &seed,
        "C2 another item's sync",
        Call::Sync(Some(ITEM_Y)),
        Clear,
    );
    cell(row, &seed, "C2 the primary's sync", Call::Sync(None), Clear);
    cell(row, &seed, "C3 a whole refresh", refresh, Begun);
    cell(row, &seed, "C3 a folder's refresh", folder, Clear);
    cell(row, &seed, "C3 a poll", Call::Poll, Clear);
    cell(
        row,
        &seed,
        "C4 another item's retirement",
        Call::Retire(ITEM_Y),
        Clear,
    );
    cell(
        row,
        &seed,
        "C4 its item's retirement",
        Call::Retire(ITEM_X),
        held(),
    );

    // R5, R6: the rows of the whole repository hold every call, as today.
    for index in 0..repository_wide_rows().len() {
        let seed = || repository_wide_rows().swap_remove(index).1;
        let (row, _, named) = repository_wide_rows().swap_remove(index);
        let refresh_held = || Refused(named.then_some(ROW_1));
        cell(row, &seed, "C1/C2 an item's write", other_write, held());
        cell(
            row,
            &seed,
            "C2 an item's write reaching other items",
            other_across,
            held(),
        );
        cell(
            row,
            &seed,
            "C2 an item's sync",
            Call::Sync(Some(ITEM_Y)),
            held(),
        );
        cell(
            row,
            &seed,
            "C2 the primary's sync",
            Call::Sync(None),
            held(),
        );
        cell(row, &seed, "C3 a whole refresh", refresh, refresh_held());
        cell(row, &seed, "C3 a rebuild", rebuild, held());
        cell(row, &seed, "C3 a folder's refresh", folder, refresh_held());
        cell(row, &seed, "C3 a poll", Call::Poll, held());
        cell(row, &seed, "C4 a retirement", Call::Retire(ITEM_Y), held());
    }

    let mismatches: Vec<String> = cases
        .into_iter()
        .filter_map(|(what, seeds, call, expected)| {
            let got = answer(&seeds, call);
            (got != expected).then(|| format!("{what}: expected {expected:?}, today {got:?}"))
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
