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
/// operations had IDs), action, target, the item it names, and its state.
struct Seed {
    id: Option<&'static str>,
    action: &'static str,
    target: String,
    item_id: Option<&'static str>,
    state: &'static str,
}

/// What a call is.
#[derive(Clone, Copy)]
enum Call {
    /// A local operation of `operation`, under `id`, with `target`.
    Begin(RepositoryOperation, &'static str, Target),
    /// The question a folder's creation asks before it makes anything:
    /// can a refresh under `id` begin?
    RefreshCanBegin(&'static str),
}

#[derive(Clone, Copy)]
enum Target {
    Ticket(&'static str),
    Document(&'static str),
    Release(&'static str),
    Prepare(&'static str),
    Repair(&'static str),
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
            Self::Empty => String::new(),
            Self::Remote => "remote-v1-0123".to_owned(),
        }
    }
}

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
                 observed_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, 7)",
            params![
                ROOT,
                seed.id,
                seed.action,
                seed.target,
                seed.item_id,
                seed.state
            ],
        )
        .unwrap();
    }
    db
}

fn answer(seeds: &[Seed], call: Call) -> Answer {
    let mut db = journal(seeds);
    let root = Path::new(ROOT);
    let result = match call {
        Call::Begin(operation, id, target) => begin_or_reconcile_operation(
            &mut db,
            root,
            operation,
            OperationId::parse(id).unwrap(),
            &target.text(),
            7,
        )
        .map(|record| {
            if record.is_new {
                Answer::Begun
            } else {
                assert!(record.is_pending, "a call that neither began nor took up");
                Answer::TakenUp
            }
        }),
        Call::RefreshCanBegin(id) => require_refresh_can_begin(
            &db,
            root,
            RepositoryOperation::CreateFolder,
            OperationId::parse(id).unwrap(),
        )
        .map(|()| Answer::Clear),
    };
    match result {
        Ok(answer) => answer,
        Err(error) => {
            assert_eq!(
                error.kind,
                RepositoryErrorKind::RecoveryRequired,
                "{error:?}"
            );
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
    use Answer::{Begun, Clear, Refused, TakenUp};
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
