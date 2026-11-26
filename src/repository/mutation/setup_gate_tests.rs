//! The setup gate (Wave 06 Cycle 02, design Part B): what a pending setup
//! refuses, where, and what it leaves alone.

use git2::Oid;
use rusqlite::Connection;

use crate::{
    canonical::{self, ItemId},
    repository::{
        CancellationToken, DocumentPromoteInput, FailurePoint, IndexRefreshInput, Mutation,
        MutationCall, MutationDataDto, NoProgress, REGISTRY_FILE, RelationshipWrite,
        RemoteRemoveInput, RepositoryService, RequestId, TicketCloseInput, TicketCreateInput,
        TicketDraft, TicketSaveInput,
        command_index_tests::{Fixture, TICKET, item},
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        open_registry,
        setup::record::{self, SetupKind, SetupRecord},
    },
    results::{Envelope, FailureClass, Outcome, ResultCode},
};

const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const SETUP_REQUEST: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
/// A document on the primary of the fixture.
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
/// Tickets nothing holds until a create makes them.
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
const NEWER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
const NEWEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE3";

struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a local write asks for no passphrase")
    }
}

fn oid(digit: char) -> Oid {
    Oid::from_str(&digit.to_string().repeat(40)).unwrap()
}

fn db(f: &Fixture) -> Connection {
    open_registry(&f.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap()
}

fn count(f: &Fixture, table: &str) -> i64 {
    db(f)
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// The setup record of the fixture's repository, made as the later tasks
/// make it, but for the commit the fixture does not need.
fn record(kind: SetupKind, primary: &str, base: Option<Oid>) -> SetupRecord {
    SetupRecord {
        setup_id: SETUP.parse::<ItemId>().unwrap(),
        kind,
        remote_name: "origin".into(),
        primary_branch: primary.into(),
        endpoint_digest: [7; 32],
        configuration: format!(
            "format_version = 2\nprimary_branch = \"{primary}\"\npublication_remote = \"origin\"\n"
        )
        .into_bytes(),
        base_configuration_oid: base,
        base_oid: None,
        commit_oid: None,
        published_oid: None,
        request_id: RequestId::parse(SETUP_REQUEST).unwrap(),
        created_at: 1_700_000_000,
    }
}

/// Replaces the repository's record with `record`.
fn set(f: &Fixture, record: &SetupRecord) {
    set_in(&f.data.path().join(REGISTRY_FILE), record);
}

fn set_in(registry: &std::path::Path, record: &SetupRecord) {
    let mut db = open_registry(registry, &mut |_| {}).unwrap();
    let id: i64 = db
        .query_row("SELECT id FROM repositories", [], |row| row.get(0))
        .unwrap();
    let tx = db.transaction().unwrap();
    record::delete(&tx, id).unwrap();
    record::insert(&tx, id, record).unwrap();
    tx.commit().unwrap();
}

fn clear(f: &Fixture) {
    let mut db = db(f);
    let tx = db.transaction().unwrap();
    tx.execute("DELETE FROM repository_setups", []).unwrap();
    tx.commit().unwrap();
}

fn create(f: &Fixture, id: &str) -> Mutation {
    Mutation::TicketCreate(TicketCreateInput {
        root: f.root.clone(),
        id: item(id),
        draft: TicketDraft {
            title: "Created".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body.\n".to_owned(),
        },
        deps: Vec::new(),
        parent: None,
        initials: Some("tt".to_owned()),
    })
}

fn save(f: &Fixture, title: &str) -> Mutation {
    let repository = f.service.resolve_repository(&f.root).unwrap();
    let observation = f
        .service
        .show_item(&repository, &item(TICKET))
        .unwrap()
        .observation
        .unwrap();
    Mutation::TicketSave(TicketSaveInput {
        root: f.root.clone(),
        id: item(TICKET),
        draft: TicketDraft {
            title: title.to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body.\n".to_owned(),
        },
        deps: RelationshipWrite::Unchanged,
        parent: RelationshipWrite::Unchanged,
        observation,
    })
}

fn close(f: &Fixture) -> Mutation {
    Mutation::TicketClose(TicketCloseInput {
        root: f.root.clone(),
        id: item(TICKET),
    })
}

fn promote(f: &Fixture) -> Mutation {
    Mutation::DocumentPromote(DocumentPromoteInput {
        root: f.root.clone(),
        id: item(DOCUMENT),
    })
}

fn remove_remote(f: &Fixture) -> Mutation {
    Mutation::RemoteRemove(RemoteRemoveInput {
        root: f.root.clone(),
        name: "origin".to_owned(),
    })
}

fn run(
    service: &RepositoryService,
    request: RequestId,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    service.execute(
        MutationCall {
            request_id: request,
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

fn ticket_exists(f: &Fixture, id: &str) -> bool {
    f.root
        .join(format!(".manyhands/tickets/{id}/ticket.md"))
        .exists()
        || f.worktree(id).exists()
}

/// A refusal of the gate: the envelope's own code, blocked, with no data,
/// recovering by `repo inspect`.
fn assert_pending<T>(envelope: &Envelope<T>, what: &str) {
    assert_eq!(envelope.code, ResultCode::SetupPending, "{what}");
    assert_eq!(envelope.outcome, Outcome::Blocked, "{what}");
    // Exit class 3.
    assert_eq!(
        envelope.failure_class(),
        Some(FailureClass::Blocked),
        "{what}"
    );
    assert!(envelope.data.is_none(), "{what}: carries no data");
    let actions: Vec<_> = envelope
        .recovery
        .iter()
        .map(|action| action.action.as_str())
        .collect();
    assert_eq!(actions, ["repo.inspect"], "{what}");
}

#[test]
fn a_pending_setup_refuses_a_first_call_an_acceptance_and_a_prepare_but_not_a_reentry() {
    let f = Fixture::new();

    // No record: nothing is refused.
    let saved = run(&f.service, RequestId::new(), save(&f, "SAVED"));
    assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");

    // A first-time record. A first call is refused with nothing written.
    set(&f, &record(SetupKind::Enable, "trunk", None));
    let (requests, confirmations) = (
        count(&f, "request_records"),
        count(&f, "confirmation_records"),
    );
    let refused = run(&f.service, RequestId::new(), create(&f, NEW));
    assert_pending(&refused, "ticket create");
    assert!(!ticket_exists(&f, NEW));
    assert_eq!(count(&f, "request_records"), requests);

    // A prepare is refused, with no confirmation made. The refusal is the
    // envelope's code, not a blocker.
    let prepared = f.service.prepare(&promote(&f));
    assert_pending(&prepared, "document promote --prepare");
    assert_eq!(count(&f, "confirmation_records"), confirmations);
    let prepared = f.service.prepare(&close(&f));
    assert_pending(&prepared, "ticket close --prepare");

    // A command that carries on is not refused.
    let refreshed = run(
        &f.service,
        RequestId::new(),
        Mutation::IndexRefresh(IndexRefreshInput {
            root: f.root.clone(),
        }),
    );
    assert_ne!(refreshed.code, ResultCode::SetupPending, "{refreshed:?}");
    // The refresh was accepted and finished: it left one record.
    let requests = count(&f, "request_records");

    // A record that appears between a first call and its acceptance
    // refuses at acceptance, and the acceptance leaves nothing behind.
    clear(&f);
    let registry = f.data.path().join(REGISTRY_FILE);
    f.service.set_acceptance_hook_for_testing(move || {
        set_in(&registry, &record(SetupKind::Enable, "trunk", None));
    });
    let refused = run(&f.service, RequestId::new(), create(&f, NEW));
    assert_pending(&refused, "ticket create at acceptance");
    assert!(!ticket_exists(&f, NEW));
    assert_eq!(count(&f, "request_records"), requests);
    assert_eq!(count(&f, "repository_setups"), 1);

    // An accepted request, re-entered while a setup is pending, is not
    // refused: it carries on to its end.
    clear(&f);
    let dying = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeRequestDomainCall,
    )
    .unwrap();
    let request = RequestId::new();
    let stopped = run(&dying, request, create(&f, NEWER));
    assert_eq!(stopped.code, ResultCode::InternalError, "{stopped:?}");
    assert_eq!(count(&f, "request_records"), requests + 1);
    set(&f, &record(SetupKind::Enable, "trunk", None));
    let finished = run(&f.service, request, create(&f, NEWER));
    assert_eq!(finished.code, ResultCode::Ok, "{finished:?}");
    assert!(ticket_exists(&f, NEWER));

    // A change record: a save carries on, a close and a remote removal
    // are refused.
    set(&f, &record(SetupKind::Change, "trunk", Some(oid('a'))));
    let saved = run(&f.service, RequestId::new(), save(&f, "Saved again"));
    assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");
    let created = run(&f.service, RequestId::new(), create(&f, NEWEST));
    assert_eq!(created.code, ResultCode::Ok, "{created:?}");
    let refused = run(&f.service, RequestId::new(), close(&f));
    assert_pending(&refused, "ticket close");
    let refused = f.service.prepare(&remove_remote(&f));
    assert_pending(&refused, "remote remove --prepare");
    let refused = run(&f.service, RequestId::new(), remove_remote(&f));
    assert_pending(&refused, "remote remove");

    // A record this build cannot read fails closed for the commands the
    // gate refuses, and lets a command that carries on through.
    set(&f, &record(SetupKind::Enable, "trunk", None));
    db(&f)
        .execute(
            "UPDATE repository_setups SET setup_id = ?1",
            ["U".repeat(26)],
        )
        .unwrap();
    for (what, mutation) in [
        ("ticket create", create(&f, "01ARZ3NDEKTSV4RRFFQ69G5FE4")),
        ("remote remove", remove_remote(&f)),
    ] {
        let refused = run(&f.service, RequestId::new(), mutation);
        assert_eq!(refused.code, ResultCode::RecoveryRequired, "{what}");
        let actions: Vec<_> = refused
            .recovery
            .iter()
            .map(|action| action.action.as_str())
            .collect();
        assert!(actions.contains(&"repo.inspect"), "{what}: {actions:?}");
    }
    let refreshed = run(
        &f.service,
        RequestId::new(),
        Mutation::IndexRefresh(IndexRefreshInput {
            root: f.root.clone(),
        }),
    );
    assert_ne!(
        refreshed.code,
        ResultCode::RecoveryRequired,
        "{refreshed:?}"
    );
    assert_ne!(refreshed.code, ResultCode::SetupPending, "{refreshed:?}");
}

#[test]
fn a_complete_setup_refuses_nothing() {
    let f = Fixture::new();
    let git = git2::Repository::open(&f.root).unwrap();
    let blob = git
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap()
        .id();

    // A first-time setup whose local primary holds a valid configuration
    // naming its primary: complete.
    set(&f, &record(SetupKind::Enable, "main", None));
    let created = run(&f.service, RequestId::new(), create(&f, NEW));
    assert_eq!(created.code, ResultCode::Ok, "{created:?}");
    let prepared = f.service.prepare(&remove_remote(&f));
    assert_ne!(prepared.code, ResultCode::SetupPending, "{prepared:?}");

    // A change whose primary's blob moved: complete. One whose blob is
    // still the base: pending.
    set(&f, &record(SetupKind::Change, "main", Some(oid('a'))));
    let prepared = f.service.prepare(&remove_remote(&f));
    assert_ne!(prepared.code, ResultCode::SetupPending, "{prepared:?}");
    set(&f, &record(SetupKind::Change, "main", Some(blob)));
    let prepared = f.service.prepare(&remove_remote(&f));
    assert_pending(&prepared, "a change whose blob is the base");

    // A primary that holds no configuration is pending even when HEAD, on
    // the setup branch, holds it: only the primary's own ref counts.
    let head = git.head().unwrap().peel_to_commit().unwrap();
    let empty = git.treebuilder(None).unwrap().write().unwrap();
    let empty = git.find_tree(empty).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let bare = git
        .commit(None, &signature, &signature, "empty", &empty, &[])
        .unwrap();
    let setup_branch = format!("refs/heads/manyhands/setup/{SETUP}");
    git.reference(&setup_branch, head.id(), true, "setup")
        .unwrap();
    git.reference("refs/heads/main", bare, true, "empty primary")
        .unwrap();
    git.set_head(&setup_branch).unwrap();
    set(&f, &record(SetupKind::Enable, "main", None));
    let prepared = f.service.prepare(&remove_remote(&f));
    assert_pending(&prepared, "only HEAD holds the configuration");
}
