//! The scope of the hold: what an unfinished row of the local journal
//! refuses. A row of one item refuses that item and, while it is mid-write,
//! work on the whole repository, not another item, whatever state its index
//! step is in. A row of the whole repository and a row whose item is not
//! known refuse everything. A refresh under a row's own ID takes that row
//! up, in that row's scope.

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
        // The guard is asked with the scope of the call: a poll is of the
        // whole repository, a synchronization of its item or the primary, a
        // retirement of its item and of the whole repository.
        Call::Poll | Call::Sync(_) | Call::Retire(_) => {
            let scope = match call {
                Call::Sync(Some(item)) => CallScope::item(&item.parse().unwrap()),
                Call::Sync(None) => CallScope::primary(),
                Call::Retire(item) => CallScope::retirement(&item.parse().unwrap()),
                _ => CallScope::repository(),
            };
            require_no_pending_local(&db, 1, &scope).map(|()| Answer::Clear)
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
    // A save of X whose commit is observed and whose hand-off failed: it
    // owes only its hand-off.
    let save_x_owing = || {
        vec![Seed {
            completed_step: Some("authoring_checkpoint_observed"),
            ..item_row(ROW_1, "save_ticket", ITEM_X, "failed")
        }]
    };
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
        // An index owner may be at work under the row with no lease held. Its
        // step stores only what a basis re-check under the lease still finds,
        // so another item's write goes on; the whole repository waits for a
        // row that has recorded no commit (Table B, E-6).
        (
            "indexing item row, another item",
            vec![item_row(ROW_1, "save_ticket", ITEM_X, "indexing")],
            Call::Begin(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
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
            Begun,
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
        // against the other tickets the index holds, and a row mid-write may
        // hold a change the index has not been told of: such a row is in its
        // way. A row that owes only its hand-off has its commit on disk,
        // where the check reads it, and is not; nor is either in the way of
        // a save that sets none.
        (
            "owing item row, another item's save setting relationships",
            save_x_owing(),
            Call::Across(SaveTicket, CALL, Ticket(ITEM_Y)),
            Begun,
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

/// Table B's rows, as the test of `holds` sorts a pending row into them.
#[derive(Clone, Copy, Debug)]
enum TableBRow {
    /// R1, R2: a per-item write whose commit is not observed.
    MidWrite,
    /// R3, R4: a per-item write, or a synchronization, whose commit is
    /// observed and that owes only its index hand-off.
    OwesOnlyItsHandOff,
    /// R5, R6: a row of the whole repository, which holds as today.
    OfTheWholeRepository,
}

/// Table B's columns, as a call stands to a pending row of one item.
#[derive(Clone, Copy, Debug)]
enum TableBColumn {
    /// C1: a write or synchronization of the row's own item, or its
    /// retirement.
    OwnItem,
    /// C2: a write or synchronization of another item.
    OtherItem,
    /// C2, for a save of another item checked against the items the index
    /// holds (`CallReach::OtherItems`), which waits for a row mid-write as
    /// vh-bhepg's rule has it ("Reads that decide on other items").
    OtherItemReachingOthers,
    /// C3: a whole-repository refresh, a poll, a rebuild, or a call that
    /// writes the primary as well (a close, a promotion).
    WholeRepository,
    /// C4: another item's retirement, and another item's `context
    /// sync-local` (design decision 19, question 4).
    OtherItemsRetirement,
}

/// Table B, cell by cell: whether a pending row holds a call.
fn table_b(row: TableBRow, column: TableBColumn) -> bool {
    use {TableBColumn::*, TableBRow::*};
    match (row, column) {
        // R5, R6: as today, in every call's way.
        (OfTheWholeRepository, _) => true,
        // C1: every row of the item holds its writes, syncs and retirement.
        (_, OwnItem) => true,
        // C2: another item's writes and syncs proceed.
        (_, OtherItem) => false,
        // C3, C4, and a save reaching other items: a row mid-write holds
        // them, and one that owes only its hand-off does not.
        (MidWrite, OtherItemReachingOthers | WholeRepository | OtherItemsRetirement) => true,
        (OwesOnlyItsHandOff, OtherItemReachingOthers | WholeRepository | OtherItemsRetirement) => {
            false
        }
    }
}

/// Where a local row in `state` whose last step is `step` stands in Table
/// B, if it is pending: its commit observed, it owes only its hand-off
/// ("`indexing`, `index_invalidated`, or a committed write before
/// indexing", the states a hand-off that did not bring the index up to
/// date leaves, and a comment that waits for its registration), or it is
/// still mid-write. `None` for a row that has ended.
fn table_b_row_of(state: &str, step: Option<&str>) -> Option<TableBRow> {
    let registering = state == "completed" && step == Some("comment_registration_pending");
    if state == "completed" && !registering {
        return None;
    }
    // The steps after which the operation's own work is done and recorded.
    let committed = matches!(
        step,
        Some(
            "authoring_checkpoint_observed"
                | "authoritative_observed"
                | "initialization_committed"
                | "publication_committed"
                | "comment_registration_pending"
                | "remote_changed"
                | "context_branch_released"
                | "context_retirement_absent"
                | "context_retirement_preserved"
        )
    );
    let handing_off = registering
        || matches!(
            state,
            "authoring_checkpoint_observed"
                | "authoritative_observed"
                | "indexing"
                | "index_invalidated"
                | "failed"
                | "retry"
        );
    Some(if committed && handing_off {
        TableBRow::OwesOnlyItsHandOff
    } else {
        TableBRow::MidWrite
    })
}

/// A call of the test of `holds`: its name, its scope, the items it is of
/// (`primary` for the primary), and the column it is of for a row of
/// another item.
type TableBCall = (
    &'static str,
    CallScope,
    &'static [&'static str],
    TableBColumn,
);

/// The calls of the test of `holds`. A close's and a promotion's scope are
/// `CallScope::of_remote`'s own.
fn table_b_calls() -> Vec<TableBCall> {
    use TableBColumn::*;
    let x = ITEM_X.parse().unwrap();
    let y = ITEM_Y.parse().unwrap();
    let plan = crate::repository::RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let lifecycle = |action, kind, item: &str| {
        CallScope::of_remote(
            &crate::repository::RemoteOperationTarget::for_context(
                &plan,
                action,
                kind,
                item.parse().unwrap(),
            )
            .unwrap(),
        )
    };
    use crate::repository::{AuthoringKind, RemoteOperationAction};
    vec![
        (
            "a write or sync of X",
            CallScope::item(&x),
            &[ITEM_X],
            OtherItem,
        ),
        (
            "a write or sync of Y",
            CallScope::item(&y),
            &[ITEM_Y],
            OtherItem,
        ),
        (
            "a save of X checked against other items",
            CallScope {
                reach: CallReach::OtherItems,
                ..CallScope::item(&x)
            },
            &[ITEM_X],
            OtherItemReachingOthers,
        ),
        (
            "a sync of the primary",
            CallScope::primary(),
            &["primary"],
            OtherItem,
        ),
        (
            "a whole refresh, a poll, a rebuild or a retirement pass",
            CallScope::repository(),
            &[],
            WholeRepository,
        ),
        (
            "a close of Z, which writes the primary",
            lifecycle(RemoteOperationAction::Close, AuthoringKind::Ticket, ITEM_Z),
            &[ITEM_Z, "primary"],
            WholeRepository,
        ),
        (
            "a close of X, its own item, which writes the primary",
            lifecycle(RemoteOperationAction::Close, AuthoringKind::Ticket, ITEM_X),
            &[ITEM_X, "primary"],
            WholeRepository,
        ),
        (
            "a promotion of X, its own item, which writes the primary",
            lifecycle(
                RemoteOperationAction::Promote,
                AuthoringKind::Document,
                ITEM_X,
            ),
            &[ITEM_X, "primary"],
            WholeRepository,
        ),
        (
            "a folder's creation, which writes the primary",
            CallScope::repository().and_the_primary(),
            &["primary"],
            WholeRepository,
        ),
        (
            "a retirement of X",
            CallScope::retirement(&x),
            &[ITEM_X],
            OtherItemsRetirement,
        ),
        (
            "context sync-local of X",
            CallScope::item(&x).and_the_repository(),
            &[ITEM_X],
            OtherItemsRetirement,
        ),
    ]
}

/// A row the test of `holds` seeds, in every state and step: its action, the
/// item it names, its target, and whose it is (the item's, `primary`, or
/// `None` for the whole repository's).
type HeldRow = (
    &'static str,
    Option<&'static str>,
    Option<String>,
    Option<&'static str>,
);

/// The one decision: `holds` gives Table B's answer for every state and
/// step a row can hold, for a row of an item, of the primary (a
/// synchronization's hand-off, P-4) and of the whole repository, against
/// every kind of call; `holds_sql` selects exactly the rows `holds` holds,
/// its columns named alone or by an alias; and
/// `holds_remote_index_debt_sql` holds a call behind a remote operation's
/// index debt as `holds` holds it behind a row owing only its hand-off of
/// the same item, or of the whole repository.
#[test]
fn holds_agrees_with_table_b() {
    let states = [
        "created",
        "branch_observed",
        "worktree_observed",
        "comment_destination_prepared",
        "authoring_destination_observed",
        "comment_checkpoint_intent",
        "authoring_checkpoint_observed",
        "authoritative_observed",
        "indexing",
        "index_invalidated",
        "failed",
        "retry",
        "observed",
        "persisted",
        "remote_changed",
        "configuration_written",
        "initialization_committed",
        "publication_committed",
        "local_sync_prepared",
        "context_worktree_released",
        "completed",
        "a_state_of_a_later_build",
    ];
    let steps: Vec<Option<&str>> = std::iter::once(None)
        .chain(LocalStep::ALL.into_iter().map(|step| Some(step.as_str())))
        .chain([Some("a_step_of_a_later_build")])
        .collect();
    let empty = || Some(String::new());
    let mut rows: Vec<HeldRow> = ITEM_ACTIONS
        .into_iter()
        .map(|action| (action, Some(ITEM_X), empty(), Some(ITEM_X)))
        .collect();
    let document_index = format!("synchronization-index-v1/document/{ITEM_X}/{OID}");
    rows.extend([
        ("save_ticket", Some(ITEM_Y), empty(), Some(ITEM_Y)),
        ("save_ticket", None, empty(), None),
        ("refresh", None, empty(), None),
        ("rebuild", None, empty(), None),
        ("context_sync_local", Some(ITEM_X), empty(), None),
        ("retire_context", None, empty(), None),
        ("enable", None, empty(), None),
        ("add_remote", None, empty(), None),
        // A synchronization's index hand-off (P-4): of its item, or of the
        // primary, whatever item it names.
        (
            "refresh",
            Some(ITEM_X),
            Some(Target::SyncIndex(ITEM_X).text()),
            Some(ITEM_X),
        ),
        ("refresh", Some(ITEM_X), Some(document_index), Some(ITEM_X)),
        (
            "refresh",
            None,
            Some(Target::PrimaryIndex.text()),
            Some("primary"),
        ),
        (
            "refresh",
            Some(ITEM_X),
            Some(Target::PrimaryIndex.text()),
            Some("primary"),
        ),
        // Of the whole repository: a hand-off target of another item than
        // the row names, or of none; the same target under another action;
        // a target that only begins like one; an older build's hand-off
        // with no target; and a synchronization with no remote, bound
        // locally.
        (
            "refresh",
            Some(ITEM_Y),
            Some(Target::SyncIndex(ITEM_X).text()),
            None,
        ),
        (
            "refresh",
            None,
            Some(Target::SyncIndex(ITEM_X).text()),
            None,
        ),
        ("rebuild", None, Some(Target::PrimaryIndex.text()), None),
        (
            "refresh",
            Some(ITEM_X),
            Some(format!("synchronization-index-v1/comment/{ITEM_X}/{OID}")),
            None,
        ),
        (
            "refresh",
            None,
            Some("synchronization-index-v1/primary".to_owned()),
            None,
        ),
        ("refresh", Some(ITEM_X), None, None),
        (
            "refresh",
            None,
            Some(format!("synchronization-local-v1/ticket/{ITEM_X}/{OID}")),
            None,
        ),
    ]);

    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE operation_records(id INTEGER PRIMARY KEY, action TEXT NOT NULL,
             target TEXT, item_id TEXT, state TEXT NOT NULL, completed_step TEXT);
         CREATE TABLE remote_operation_records(id INTEGER PRIMARY KEY, action TEXT NOT NULL,
             item_id TEXT, index_pending INTEGER NOT NULL);",
    )
    .unwrap();
    let mut seeded = Vec::new();
    for (action, item, target, whose) in &rows {
        for state in states {
            for &step in &steps {
                db.execute(
                    "INSERT INTO operation_records(action, target, item_id, state, completed_step)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![action, target, item, state, step],
                )
                .unwrap();
                let row = PendingRow {
                    action: (*action).to_owned(),
                    target: target.clone(),
                    item_id: item.map(str::to_owned),
                    state: state.to_owned(),
                    completed_step: step.map(str::to_owned),
                };
                seeded.push((db.last_insert_rowid(), *whose, row));
            }
        }
    }
    let selected = |sql: &str| -> std::collections::BTreeSet<i64> {
        db.prepare(sql)
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };

    for (call_name, call, of, column) in table_b_calls() {
        let mut mismatches = Vec::new();
        let mut held = std::collections::BTreeSet::new();
        for (id, whose, row) in &seeded {
            let expected = table_b_row_of(&row.state, row.completed_step.as_deref()).map(|cell| {
                let cell = if whose.is_none() {
                    TableBRow::OfTheWholeRepository
                } else {
                    cell
                };
                let column = if whose.is_some_and(|whose| of.contains(&whose)) {
                    TableBColumn::OwnItem
                } else {
                    column
                };
                table_b(cell, column)
            });
            let got = holds(row, &call);
            if got != expected.unwrap_or(false) {
                mismatches.push(format!("{row:?}: Table B {expected:?}, holds {got}"));
            }
            if got {
                held.insert(*id);
            }
        }
        assert!(
            mismatches.is_empty(),
            "{call_name}:\n{}",
            mismatches.join("\n")
        );
        assert_eq!(
            selected(&format!(
                "SELECT id FROM operation_records WHERE {}",
                holds_sql(&call, "")
            )),
            held,
            "{call_name}: holds_sql"
        );
        assert_eq!(
            selected(&format!(
                "SELECT o.id FROM operation_records o WHERE {}",
                holds_sql(&call, "o.")
            )),
            held,
            "{call_name}: holds_sql by an alias"
        );
    }

    // A remote operation's index debt: a synchronization's owes only the
    // hand-off of its item or of the primary; any other remote operation's
    // is a whole-repository refresh. Each is held as `holds` holds a row
    // of the same scope owing only its hand-off.
    let owing = |item: Option<&str>| PendingRow {
        action: "save_ticket".to_owned(),
        target: None,
        item_id: item.map(str::to_owned),
        state: "authoritative_observed".to_owned(),
        completed_step: Some("authoring_checkpoint_observed".to_owned()),
    };
    let mut remote = Vec::new();
    for (action, item, scope) in [
        ("synchronize_context", Some(ITEM_X), Some(ITEM_X)),
        ("synchronize_context", Some(ITEM_Y), Some(ITEM_Y)),
        ("synchronize_primary", None, Some("primary")),
        ("poll", None, None),
        ("promote", Some(ITEM_Z), None),
        ("close", Some(ITEM_Z), None),
    ] {
        for index_pending in [0, 1] {
            db.execute(
                "INSERT INTO remote_operation_records(action, item_id, index_pending)
                 VALUES (?1, ?2, ?3)",
                params![action, item, index_pending],
            )
            .unwrap();
            remote.push((db.last_insert_rowid(), scope, index_pending != 0));
        }
    }
    for (call_name, call, _, _) in table_b_calls() {
        let held: std::collections::BTreeSet<i64> = remote
            .iter()
            .filter(|(_, scope, debt)| *debt && holds(&owing(*scope), &call))
            .map(|(id, _, _)| *id)
            .collect();
        for columns in ["", "r."] {
            assert_eq!(
                selected(&format!(
                    "SELECT r.id FROM remote_operation_records r WHERE {}",
                    holds_remote_index_debt_sql(&call, columns)
                )),
                held,
                "{call_name}: holds_remote_index_debt_sql ({columns:?})"
            );
        }
    }
}

// Table B of the fast-writes design (`docs/plans/2026-11-19-fast-writes-design.md`,
// "Table B: what a pending row holds"), written before the behaviour it
// pins (Task 3). The rows of a synchronization's index hand-off are asked in
// a test of their own, written as Task 16 writes them, as rows of their item
// (P-4). A row holds a call
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

/// The states of Table B's R3 rows, a write that owes only its hand-off,
/// its commit observed: a committed write whose state has not advanced yet
/// (state and step `authoring_checkpoint_observed`, which a crash between
/// the checkpoint and the hand-off leaves; owing, by design decision 19,
/// question 1), the hand-off begun before indexing, a claim by an index
/// owner (live or crashed), a fence, and what a hand-off that did not bring
/// the index up to date leaves.
const R3_STATES: [&str; 5] = [
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

/// The calls of Table B's columns, for a row of item X.
const OWN_WRITE: Call = Call::Begin(SaveTicket, CALL, Target::Ticket(ITEM_X));
const OTHER_WRITE: Call = Call::Begin(SaveTicket, CALL, Target::Ticket(ITEM_Y));
const OTHER_ACROSS: Call = Call::Across(SaveTicket, CALL, Target::Ticket(ITEM_Y));
const REFRESH: Call = Call::Begin(RefreshRepository, CALL, Target::Empty);
const REBUILD: Call = Call::Begin(RebuildRepository, CALL, Target::Empty);
const FOLDER: Call = Call::RefreshCanBegin(CALL);
const RESUME: Call = Call::Begin(RefreshRepository, ROW_1, Target::Empty);

/// The answer of a call a row holds: refused, naming the row.
fn held() -> Answer {
    Answer::Refused(Some(ROW_1))
}

/// The cells of a Table B table: each one's name, the rows it seeds, its
/// call and the answer Table B gives it.
#[derive(Default)]
struct Cells(Vec<(String, Vec<Seed>, Call, Answer)>);

impl Cells {
    fn cell(
        &mut self,
        row: &str,
        seed: &dyn Fn() -> Seed,
        column: &str,
        call: Call,
        expected: Answer,
    ) {
        self.0
            .push((format!("{row} | {column}"), vec![seed()], call, expected));
    }

    /// Asks each cell's call, and fails listing every cell whose answer is
    /// not Table B's.
    fn assert_answers(self) {
        let mismatches: Vec<String> = self
            .0
            .into_iter()
            .filter_map(|(what, seeds, call, expected)| {
                let got = answer(&seeds, call);
                (got != expected).then(|| format!("{what}: expected {expected:?}, got {got:?}"))
            })
            .collect();
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }
}

/// Table B's rows of local operations: a per-item write mid-write (R1, R2),
/// one that owes only its hand-off (R3), a comment waiting for its
/// registration (R3), and the rows of the whole repository (R5, R6).
#[test]
fn table_b_what_a_pending_row_holds_at_the_local_begin_and_the_remote_guard() {
    use Answer::{Begun, Clear, Refused, TakenUp};
    use Target::Ticket;
    let (own_write, other_write, other_across) = (OWN_WRITE, OTHER_WRITE, OTHER_ACROSS);
    let (refresh, rebuild, folder, resume) = (REFRESH, REBUILD, FOLDER, RESUME);
    let mut cells = Cells::default();
    let mut cell = |row: &str, seed: &dyn Fn() -> Seed, column: &str, call: Call, expected| {
        cells.cell(row, seed, column, call, expected);
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
    // A synchronization admitted past the row begins its hand-off row of
    // its own item, or of the primary, past it too (carry C-4).
    cell(
        row,
        &seed,
        "C2 another item's sync hand-off",
        Call::Begin(RefreshRepository, CALL, Target::SyncIndex(ITEM_Y)),
        Begun,
    );
    cell(
        row,
        &seed,
        "C2 the primary's sync hand-off",
        Call::Begin(RefreshRepository, CALL, Target::PrimaryIndex),
        Begun,
    );
    cell(
        row,
        &seed,
        "C1 its item's sync hand-off",
        Call::Begin(RefreshRepository, CALL, Target::SyncIndex(ITEM_X)),
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
    for state in R3_STATES {
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

    cells.assert_answers();
}

/// Table B's rows of a synchronization's index hand-off (R4, and R4 with
/// Owner question 2 for the primary's), which Task 16's hand-off rows
/// (P-4) are written for.
#[test]
fn table_b_what_a_synchronization_s_hand_off_holds_at_the_local_begin_and_the_remote_guard() {
    use Answer::{Begun, Clear, TakenUp};
    use Target::{PrimaryIndex, SyncIndex};
    let (own_write, other_write, other_across) = (OWN_WRITE, OTHER_WRITE, OTHER_ACROSS);
    let (refresh, rebuild, folder) = (REFRESH, REBUILD, FOLDER);
    let mut cells = Cells::default();
    let mut cell = |row: &str, seed: &dyn Fn() -> Seed, column: &str, call: Call, expected| {
        cells.cell(row, seed, column, call, expected);
    };

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

    cells.assert_answers();
}
