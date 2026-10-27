//! `context prepare` and `context release` through the boundary: the work
//! context (a branch and a worktree) an item is edited in, provisioned for
//! an existing item or a new one, and removed again when it holds no work.
//! Every row of the design's two outcome tables is a test here, written
//! before the command was: the `prepare_` tests are the first table's and
//! the `release_` tests the second's.
//!
//! The last row of the first table ends in `context release`:
//! `prepare_abandoning_the_interrupted_row_and_moving_primary_leaves_a_branch_edit_refuses`
//! drives the whole row, the release of the stranded branch included.

use std::fs;

use manyhands::{
    repository::{
        AddRemoteRequest, AuthoringKind, CloseTicketRequest, ContextPrepareInput,
        ContextPrepareIntent, ContextReleaseInput, FailurePoint, Mutation, MutationDataDto,
        OperationAbandonInput, OperationId, OperationResumeInput, PreviewTicketClosureRequest,
        RemoteOperationAction, RemoteOperationTarget, RemotePublicationEvidence, RemoteRefPlan,
        RemoteReservationOutcome, RequestState, SetPublicationRemoteRequest,
        TicketClosurePreflight, keys::SessionCredentials,
    },
    results::{DiscoveryEffect, Effects, Envelope, Outcome, ResultCode},
};

use super::support::mutation::Previewed as _;
use super::support::{
    FailOnce, git_home,
    golden::ContractCase,
    items,
    mutation::{
        self, NoPassphrase, REQUEST_1, REQUEST_2, REQUEST_3, SENTINELS, TICKET_CLOSED, TICKET_NEW,
        World, recovery_actions,
    },
};

const CONTEXT_SCHEMA: &str = "context_mutation.schema.json";

/// More request IDs than the support module has. No fixture uses them.
const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
const REQUEST_5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";

fn prepare(world: &World, kind: AuthoringKind, id: &str, intent: ContextPrepareIntent) -> Mutation {
    Mutation::ContextPrepare(ContextPrepareInput {
        root: world.root.clone(),
        kind,
        id: items::item_id(id),
        intent,
    })
}

fn edit_ticket(world: &World, id: &str) -> Mutation {
    prepare(world, AuthoringKind::Ticket, id, ContextPrepareIntent::Edit)
}

fn create_ticket(world: &World, id: &str) -> Mutation {
    prepare(
        world,
        AuthoringKind::Ticket,
        id,
        ContextPrepareIntent::Create,
    )
}

fn create_document(world: &World, id: &str) -> Mutation {
    prepare(
        world,
        AuthoringKind::Document,
        id,
        ContextPrepareIntent::Create,
    )
}

/// A world whose service fails once at `point`.
fn failing_at(point: FailurePoint) -> World {
    World::with_service(|data| FailOnce::at(point).open_service(data))
}

fn discovery(effect: DiscoveryEffect) -> Effects {
    Effects {
        discovery: effect,
        ..Effects::not_requested()
    }
}

/// The data of a `context prepare` envelope: id, kind, branch, worktree
/// and whether the request made the context.
fn context(envelope: &Envelope<MutationDataDto>) -> (String, String, String, String, bool) {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Context(data)) => (
            data.id.clone(),
            data.kind.clone(),
            data.branch.clone(),
            data.worktree.clone(),
            data.created,
        ),
        other => panic!("not a context's data: {other:?}"),
    }
}

fn created(envelope: &Envelope<MutationDataDto>) -> bool {
    context(envelope).4
}

fn assert_ok(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok),
        "{envelope:?}"
    );
    assert_eq!(envelope.effects, discovery(DiscoveryEffect::Current));
}

/// The names of the branches `manyhands/*` and of the directories under
/// the worktrees directory.
fn contexts(world: &World) -> (Vec<String>, Vec<String>) {
    let repository = git2::Repository::open(&world.root).unwrap();
    let mut branches: Vec<String> = repository
        .branches(Some(git2::BranchType::Local))
        .unwrap()
        .map(|branch| branch.unwrap().0.name().unwrap().unwrap().to_owned())
        .filter(|name| name.starts_with("manyhands/"))
        .collect();
    branches.sort();
    let mut worktrees: Vec<String> = fs::read_dir(world.root.join(".manyhands/worktrees"))
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    worktrees.sort();
    (branches, worktrees)
}

/// Asserts that exactly the context of the ticket `id` is there.
fn assert_one_context(world: &World, id: &str) {
    assert_eq!(
        contexts(world),
        (vec![World::branch(id)], vec![id.to_owned()])
    );
    assert!(world.worktree(id).is_dir());
}

/// Every row of the local journal: operation ID, action, state, step.
fn journal(world: &World) -> Vec<(String, String, String, Option<String>)> {
    items::index(world.data.path())
        .prepare(
            "SELECT operation_ulid, action, state, completed_step
               FROM operation_records ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn operation_of(world: &World, request_id: &str) -> OperationId {
    world.record(request_id).unwrap().operations[0].operation_id
}

fn assert_golden(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    let root = world.root.to_str().unwrap().to_owned();
    let operation = envelope.operation_id.clone();
    let mut placeholders = vec![(root.as_str(), "<root>")];
    if let Some(operation) = operation.as_deref() {
        placeholders.push((operation, "<operation>"));
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: Some(CONTEXT_SCHEMA),
            placeholders: &placeholders,
            sentinels: &[world.data.path().to_str().unwrap()],
        },
        envelope,
    );
}

#[test]
fn prepare_edit_provisions_and_a_second_request_reuses_it() {
    git_home::isolated();
    let world = World::new();
    let head = world.primary_head();

    let first = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_ok(&first);
    assert_eq!(
        context(&first),
        (
            items::TICKET_A.to_owned(),
            "ticket".to_owned(),
            World::branch(items::TICKET_A),
            world.worktree(items::TICKET_A).to_str().unwrap().to_owned(),
            true
        )
    );
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(world.branch_tip(items::TICKET_A), Some(head));
    assert_eq!(world.primary_head(), head);
    assert_eq!(first.scope.item_id.as_deref(), Some(items::TICKET_A));
    assert_eq!(
        first.scope.branch.as_deref(),
        Some(World::branch(items::TICKET_A).as_str())
    );
    assert!(first.operation_id.is_some());
    assert_golden("context_prepare", &world, &first);

    // Another request finds the context and makes nothing.
    let rows = journal(&world).len();
    let second = world.execute(REQUEST_2, edit_ticket(&world, items::TICKET_A));
    assert_ok(&second);
    assert!(!created(&second), "{second:?}");
    assert_eq!(context(&second).2, World::branch(items::TICKET_A));
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(journal(&world).len(), rows + 1);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn prepare_create_provisions_for_a_new_id_and_document_create_reuses_it() {
    git_home::isolated();
    let world = World::new();

    let prepared = world.execute(REQUEST_1, create_document(&world, mutation::DOCUMENT_NEW));
    assert_ok(&prepared);
    let (id, kind, branch, _, made) = context(&prepared);
    assert_eq!(
        (id.as_str(), kind.as_str(), made),
        (mutation::DOCUMENT_NEW, "document", true)
    );
    assert_eq!(branch, World::document_branch(mutation::DOCUMENT_NEW));
    assert_eq!(
        contexts(&world),
        (
            vec![World::document_branch(mutation::DOCUMENT_NEW)],
            vec![mutation::DOCUMENT_NEW.to_owned()]
        )
    );

    let saved = world.execute(
        REQUEST_2,
        world.document_create(mutation::DOCUMENT_NEW, mutation::DOCUMENT_NEW_PATH),
    );
    assert_eq!(
        (saved.outcome, saved.code),
        (Outcome::Success, ResultCode::Ok),
        "{saved:?}"
    );
    // The create wrote into the context the preparation made.
    assert_eq!(
        contexts(&world),
        (
            vec![World::document_branch(mutation::DOCUMENT_NEW)],
            vec![mutation::DOCUMENT_NEW.to_owned()]
        )
    );
    assert_eq!(world.branch_commits(mutation::DOCUMENT_NEW), 1);
}

#[test]
fn prepare_create_provisions_for_a_new_id_and_ticket_create_reuses_it() {
    git_home::isolated();
    let world = World::new();

    let prepared = world.execute(REQUEST_1, create_ticket(&world, TICKET_NEW));
    assert_ok(&prepared);
    assert!(created(&prepared), "{prepared:?}");
    assert_one_context(&world, TICKET_NEW);

    let saved = world.execute(REQUEST_2, world.create(TICKET_NEW));
    assert_eq!(
        (saved.outcome, saved.code),
        (Outcome::Success, ResultCode::Ok),
        "{saved:?}"
    );
    assert_one_context(&world, TICKET_NEW);
    assert_eq!(world.branch_commits(TICKET_NEW), 1);
}

#[test]
fn prepare_edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing() {
    git_home::isolated();
    let world = World::new();
    let rows = journal(&world);

    let refused = world.execute(REQUEST_1, edit_ticket(&world, TICKET_CLOSED));

    assert_eq!(refused.code, ResultCode::TicketClosed, "{refused:?}");
    assert_eq!(refused.data, None);
    assert_eq!(refused.effects, Effects::not_requested());
    world.assert_nothing_exists(REQUEST_1, TICKET_CLOSED);
    assert_eq!(journal(&world), rows);
    assert_eq!(contexts(&world), (vec![], vec![]));
}

#[test]
fn prepare_edit_of_an_uncommitted_primary_change_is_worktree_not_clean_and_makes_nothing() {
    git_home::isolated();
    let world = World::new();
    let rows = journal(&world);
    items::write(
        &world.root,
        &items::ticket_path(items::TICKET_A),
        &items::ticket_source(items::TICKET_A, "Edited, not committed", ""),
    );

    let refused = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(refused.code, ResultCode::WorktreeNotClean, "{refused:?}");
    assert_eq!(refused.data, None);
    world.assert_nothing_exists(REQUEST_1, items::TICKET_A);
    assert_eq!(journal(&world), rows);
    assert_eq!(contexts(&world), (vec![], vec![]));
}

#[test]
fn prepare_create_of_an_existing_id_is_occupied_path() {
    git_home::isolated();
    let world = World::new();
    let rows = journal(&world);
    for (request, mutation, id) in [
        (
            REQUEST_1,
            create_ticket(&world, items::TICKET_A),
            items::TICKET_A,
        ),
        (
            REQUEST_2,
            create_document(&world, mutation::DOCUMENT),
            mutation::DOCUMENT,
        ),
        // The other kind: a document is no more free for an ID a ticket holds.
        (
            REQUEST_3,
            create_document(&world, items::TICKET_A),
            items::TICKET_A,
        ),
    ] {
        let refused = world.execute(request, mutation);
        assert_eq!(refused.code, ResultCode::OccupiedPath, "{refused:?}");
        assert_eq!(refused.data, None);
        world.assert_nothing_exists(request, id);
    }
    assert_eq!(journal(&world), rows);
}

#[test]
fn prepare_edit_of_a_missing_item_is_item_not_found() {
    git_home::isolated();
    let world = World::new();
    let rows = journal(&world);
    // An item that is no ticket's is no ticket to edit either.
    for (request, mutation, id) in [
        (REQUEST_1, edit_ticket(&world, TICKET_NEW), TICKET_NEW),
        (
            REQUEST_2,
            edit_ticket(&world, mutation::DOCUMENT),
            mutation::DOCUMENT,
        ),
    ] {
        let refused = world.execute(request, mutation);
        assert_eq!(refused.code, ResultCode::ItemNotFound, "{refused:?}");
        assert_eq!(refused.data, None);
        world.assert_nothing_exists(request, id);
    }
    assert_eq!(journal(&world), rows);
}

#[test]
fn prepare_records_hold_no_body() {
    git_home::isolated();
    let world = World::new();
    // A ticket whose context holds the sentinel title and body.
    let token = world.token(items::TICKET_A);
    let saved = world.execute(REQUEST_1, world.save(items::TICKET_A, &token));
    assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");

    let prepared = world.execute(REQUEST_2, edit_ticket(&world, items::TICKET_A));
    assert_ok(&prepared);
    assert!(!created(&prepared));

    let text = serde_json::to_string(&prepared).unwrap();
    for sentinel in SENTINELS {
        assert!(!text.contains(sentinel), "the envelope holds {sentinel}");
    }
    mutation::assert_request_stores_exclude(world.data.path(), &SENTINELS);
    let record = format!("{:?}", world.record(REQUEST_2).unwrap());
    for sentinel in SENTINELS {
        assert!(!record.contains(sentinel), "the record holds {sentinel}");
    }

    // And the path that makes the context, over a primary file whose body is
    // the sentinel.
    let world = World::new();
    let path = items::ticket_path(items::TICKET_A);
    let source = items::ticket_source(items::TICKET_A, mutation::TITLE, "");
    items::write(&world.root, &path, &format!("{source}{}", mutation::BODY));
    items::commit(&world.fixture, &[&path], items::COMMITTED_AT + 1);
    let made = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_ok(&made);
    assert!(created(&made));
    let text = serde_json::to_string(&made).unwrap();
    for sentinel in SENTINELS {
        assert!(!text.contains(sentinel), "the envelope holds {sentinel}");
    }
    mutation::assert_request_stores_exclude(world.data.path(), &SENTINELS);
    let record = format!("{:?}", world.record(REQUEST_1).unwrap());
    for sentinel in SENTINELS {
        assert!(!record.contains(sentinel), "the record holds {sentinel}");
    }
}

// The rows of the design's table, in its order.

#[test]
fn prepare_the_same_request_after_ok_is_the_stored_answer_and_calls_no_domain() {
    git_home::isolated();
    let world = World::new();
    let first = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_ok(&first);
    let rows = journal(&world);
    let record = world.record(REQUEST_1).unwrap();
    assert_eq!(record.state, RequestState::Finished);

    let again = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(
        serde_json::to_value(&again).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    assert!(created(&again));
    assert_eq!(journal(&world), rows);
    assert_eq!(world.record(REQUEST_1).unwrap().result, record.result);
}

#[test]
fn prepare_the_same_request_after_discovery_pending_finishes_the_hand_off() {
    git_home::isolated();
    let world = failing_at(FailurePoint::BeforeIndexTransactionCommit);

    let pending = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(
        (pending.outcome, pending.code),
        (Outcome::Partial, ResultCode::DiscoveryPending),
        "{pending:?}"
    );
    assert_eq!(pending.effects, discovery(DiscoveryEffect::Pending));
    // The row's step is the branch and worktree steps, not an
    // authoritative one (the journal keeps the first step it recorded),
    // so a resume leaves the hand-off to this request, and the result
    // offers the request in its place (`offering_resume`). The design's
    // prose says `operation.resume`; see the task report.
    assert_eq!(
        recovery_actions(&pending),
        ["request.retry", "repo.inspect", "operation.abandon"],
        "{pending:?}"
    );
    let resumed = world.execute(
        REQUEST_2,
        Mutation::OperationResume(OperationResumeInput {
            root: world.root.clone(),
            operation_id: operation_of(&world, REQUEST_1),
        }),
    );
    assert_eq!(
        resumed.code,
        ResultCode::OriginalRequestRequired,
        "{resumed:?}"
    );
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(world.pending_journal_rows(), 1);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
    let operation = pending.operation_id.clone().unwrap();
    assert_golden("context_prepare_discovery_pending", &world, &pending);

    let finished = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_ok(&finished);
    assert!(created(&finished), "{finished:?}");
    assert_eq!(finished.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_one_context(&world, items::TICKET_A);
}

#[test]
fn prepare_interrupted_at_the_domain_call_the_same_request_makes_the_context() {
    git_home::isolated();
    let world = failing_at(FailurePoint::BeforeRequestDomainCall);

    let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    let retry = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_ok(&retry);
    assert!(created(&retry), "{retry:?}");
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn prepare_interrupted_before_the_worktree_other_requests_need_recovery_and_the_same_one_finishes()
{
    git_home::isolated();
    let world = failing_at(FailurePoint::BeforeWorktreeCreation);

    let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    // The branch is there and the worktree is not.
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );
    assert_eq!(world.pending_journal_rows(), 1);

    // Another request, of the same item or of another, is stopped by the
    // pending row.
    for (request, mutation) in [
        (REQUEST_2, edit_ticket(&world, items::TICKET_A)),
        (REQUEST_3, create_ticket(&world, TICKET_NEW)),
    ] {
        let other = world.execute(request, mutation);
        assert_eq!(other.code, ResultCode::RecoveryRequired, "{other:?}");
    }
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );

    let retry = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_ok(&retry);
    assert!(created(&retry), "{retry:?}");
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn prepare_after_an_uncommitted_edit_or_a_closure_the_same_request_still_continues_to_the_domain() {
    git_home::isolated();
    // Both edits are uncommitted: the conditions the prechecks test change
    // without moving primary's head. A closure that is committed moves the
    // head, and then the branch left behind is stale: see
    // `prepare_interrupted_before_the_worktree_then_a_committed_closure_is_recovery_required`.
    for edit in [
        items::ticket_source(items::TICKET_A, "Edited, not committed", ""),
        items::ticket_source_with(
            items::TICKET_A,
            "Open ticket",
            "bug",
            "done",
            items::CLOSURE,
        ),
    ] {
        let world = failing_at(FailurePoint::BeforeWorktreeCreation);
        let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
        assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
        items::write(&world.root, &items::ticket_path(items::TICKET_A), &edit);

        let retry = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

        assert_ok(&retry);
        assert!(created(&retry), "{retry:?}");
        assert_one_context(&world, items::TICKET_A);
    }
}

#[test]
fn prepare_interrupted_before_settlement_the_same_request_is_ok_with_one_context() {
    git_home::isolated();
    let world = failing_at(FailurePoint::BeforeRequestSettlement);

    let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    let retry = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_ok(&retry);
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(world.pending_journal_rows(), 0);
    // The row records no checkpoint step (the journal keeps its first step),
    // so the boundary runs the request as new and finds the context: the
    // answer names what a direct Created names, but did not make it.
    let direct = World::new();
    let direct_made = direct.execute(REQUEST_1, edit_ticket(&direct, items::TICKET_A));
    let named = |world: &World, envelope: &Envelope<MutationDataDto>| {
        let (id, kind, branch, worktree, _) = context(envelope);
        let worktree = worktree
            .strip_prefix(world.root.to_str().unwrap())
            .unwrap()
            .to_owned();
        (id, kind, branch, worktree)
    };
    assert_eq!(named(&world, &retry), named(&direct, &direct_made));
    assert!(created(&direct_made));
    assert!(!created(&retry), "{retry:?}");
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn prepare_the_same_request_with_another_intent_or_item_is_a_mismatch() {
    git_home::isolated();
    let world = World::new();
    let first = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_ok(&first);
    let record = world.record(REQUEST_1);

    for changed in [
        // Another intent.
        create_ticket(&world, items::TICKET_A),
        // Another item.
        edit_ticket(&world, items::TICKET_C),
        // Another kind.
        prepare(
            &world,
            AuthoringKind::Document,
            items::TICKET_A,
            ContextPrepareIntent::Edit,
        ),
    ] {
        let mismatch = world.execute(REQUEST_1, changed);
        assert_eq!(mismatch.code, ResultCode::RequestMismatch, "{mismatch:?}");
        assert_eq!(world.record(REQUEST_1), record);
    }

    // And while the request is unfinished.
    let world = failing_at(FailurePoint::BeforeWorktreeCreation);
    world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    let mismatch = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_C));
    // An attempt of the request made the branch, so the mismatch is partial
    // (as `mutation_replay/execute.rs` has it for a commit).
    assert_eq!(
        (mismatch.outcome, mismatch.code),
        (Outcome::Partial, ResultCode::RequestMismatch),
        "{mismatch:?}"
    );

    // And after the context was made and the answer was lost. The completed row says
    // nothing of what the attempt made, so nothing is reported.
    let world = failing_at(FailurePoint::BeforeRequestSettlement);
    world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    let mismatch = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_C));
    assert_eq!(
        (mismatch.outcome, mismatch.code),
        (Outcome::Error, ResultCode::RequestMismatch),
        "{mismatch:?}"
    );
}

#[test]
fn prepare_abandoning_the_interrupted_row_and_moving_primary_leaves_a_branch_edit_refuses() {
    git_home::isolated();
    let world = failing_at(FailurePoint::BeforeWorktreeCreation);
    let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    let operation = operation_of(&world, REQUEST_1);

    // The row is abandoned: the branch is left behind, and the worktree
    // was never made.
    let abandon = || {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: world.root.clone(),
            operation_id: operation,
        })
    };
    let preview = world.service.prepare(&abandon()).previewed();
    assert_eq!(preview.code, ResultCode::Ok, "{preview:?}");
    let confirmation = preview.data.unwrap().confirmation_id;
    let abandoned =
        mutation::execute_confirmed(&world.service, REQUEST_2, &confirmation, abandon());
    assert_eq!(abandoned.code, ResultCode::Ok, "{abandoned:?}");
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );

    // Primary moves on.
    let path = items::ticket_path(items::TICKET_C);
    items::write(
        &world.root,
        &path,
        &items::ticket_source(items::TICKET_C, "Moved on", ""),
    );
    items::commit(&world.fixture, &[&path], items::COMMITTED_AT + 1);

    // The branch is no longer at primary's head: an Edit cannot use it.
    let refused = world.execute(REQUEST_3, edit_ticket(&world, items::TICKET_A));
    assert_eq!(refused.code, ResultCode::RecoveryRequired, "{refused:?}");
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );

    // `context release` removes the stranded branch, which holds no commit
    // of its own, and an Edit then makes the context afresh.
    let released = world.execute(REQUEST_4, release_ticket(&world, items::TICKET_A));
    assert_released(&released);
    assert_eq!(contexts(&world), (vec![], vec![]));
    let edit = world.execute(REQUEST_5, edit_ticket(&world, items::TICKET_A));
    assert_ok(&edit);
    assert!(created(&edit), "{edit:?}");
    assert_one_context(&world, items::TICKET_A);
}

#[test]
fn prepare_interrupted_before_the_worktree_then_a_committed_closure_is_recovery_required() {
    git_home::isolated();
    let world = failing_at(FailurePoint::BeforeWorktreeCreation);
    let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");

    // The closure is committed on primary: its head moves past the branch
    // the interrupted call cut.
    let path = items::ticket_path(items::TICKET_A);
    items::write(
        &world.root,
        &path,
        &items::ticket_source_with(
            items::TICKET_A,
            "Open ticket",
            "bug",
            "done",
            items::CLOSURE,
        ),
    );
    items::commit(&world.fixture, &[&path], items::COMMITTED_AT + 1);

    let retry = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_eq!(retry.code, ResultCode::RecoveryRequired, "{retry:?}");
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );
}

// `context release`: the helpers, then the rows of the design's table in
// its order.

fn release(world: &World, kind: AuthoringKind, id: &str) -> Mutation {
    Mutation::ContextRelease(ContextReleaseInput {
        root: world.root.clone(),
        kind,
        id: items::item_id(id),
    })
}

fn release_ticket(world: &World, id: &str) -> Mutation {
    release(world, AuthoringKind::Ticket, id)
}

fn assert_released(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok),
        "{envelope:?}"
    );
    assert_eq!(envelope.effects, discovery(DiscoveryEffect::Current));
    assert_eq!(envelope.data, None);
}

/// Asserts that the request was refused with `code`, carrying nothing,
/// and that nothing is left of it: no record and no pending row.
fn assert_refused(
    world: &World,
    request_id: &str,
    envelope: &Envelope<MutationDataDto>,
    code: ResultCode,
) {
    assert_eq!(envelope.code, code, "{envelope:?}");
    assert_eq!(envelope.data, None);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(world.record(request_id).is_none(), "a request record");
    assert_eq!(world.pending_journal_rows(), 0, "a pending journal row");
}

/// Whether the index lists the context of the branch `branch`.
fn indexed(world: &World, branch: &str) -> bool {
    items::index(world.data.path())
        .query_row(
            "SELECT COUNT(*) FROM contexts WHERE kind = 'active' AND branch = ?1",
            [branch],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
        == 1
}

/// The repository's references, by name.
fn references(world: &World) -> References {
    let repository = git2::Repository::open(&world.root).unwrap();
    let mut references: Vec<_> = repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            (reference.name().unwrap().to_owned(), reference.target())
        })
        .collect();
    references.sort();
    references
}

/// The branches and the worktree directories of the contexts.
type Contexts = (Vec<String>, Vec<String>);

/// Every reference of the repository and where it points.
type References = Vec<(String, Option<git2::Oid>)>;

/// What a refusal must leave as it found it: the contexts, every
/// reference, and the index's row of the ticket's context.
fn image(world: &World, id: &str) -> (Contexts, References, bool) {
    (
        contexts(world),
        references(world),
        indexed(world, &World::branch(id)),
    )
}

/// Makes the ticket's context through the boundary, at primary's head.
fn make(world: &World, request_id: &str, id: &str) {
    let made = world.execute(request_id, edit_ticket(world, id));
    assert_ok(&made);
    assert!(created(&made), "{made:?}");
    assert!(indexed(world, &World::branch(id)));
}

/// The world's service, from now on, fails once at `point`.
fn fail_once_at(world: &mut World, point: FailurePoint) {
    world.service = FailOnce::at(point).open_service(world.data.path());
}

fn assert_release_golden(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    let root = world.root.to_str().unwrap().to_owned();
    let operation = envelope.operation_id.clone().unwrap();
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: None,
            placeholders: &[
                (root.as_str(), "<root>"),
                (operation.as_str(), "<operation>"),
            ],
            sentinels: &[world.data.path().to_str().unwrap()],
        },
        envelope,
    );
}

/// Adds the remote `origin`, which no test reaches, and selects it for
/// publication. Both are local: nothing is fetched.
fn select_origin(world: &World) {
    world
        .service
        .add_remote(AddRemoteRequest {
            root: world.root.clone(),
            name: "origin".to_owned(),
            url: "ssh://fixture@localhost/repository".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    select(world, Some("origin"));
}

fn select(world: &World, name: Option<&str>) {
    world
        .service
        .set_publication_remote(SetPublicationRemoteRequest {
            root: world.root.clone(),
            name: name.map(str::to_owned),
            operation_id: OperationId::new(),
        })
        .unwrap();
}

fn evidence(world: &World, id: &str) -> RemotePublicationEvidence {
    world
        .service
        .remote_snapshot(&world.root)
        .unwrap()
        .publication_evidence_for(AuthoringKind::Ticket, &items::item_id(id))
}

#[test]
fn release_with_no_saved_work_removes_the_context_and_its_index_row_and_nothing_else() {
    git_home::isolated();
    let world = World::new();
    let head = world.primary_head();
    // A remote ref that is no context's.
    git2::Repository::open(&world.root)
        .unwrap()
        .reference("refs/remotes/origin/main", head, false, "fixture")
        .unwrap();
    make(&world, REQUEST_1, items::TICKET_A);
    // Another context, which stays.
    let other = world.execute(
        REQUEST_2,
        prepare(
            &world,
            AuthoringKind::Document,
            mutation::DOCUMENT,
            ContextPrepareIntent::Edit,
        ),
    );
    assert_ok(&other);
    let other_branch = World::document_branch(mutation::DOCUMENT);
    let mut kept = references(&world);
    kept.retain(|(name, _)| *name != format!("refs/heads/{}", World::branch(items::TICKET_A)));

    let released = world.execute(REQUEST_3, release_ticket(&world, items::TICKET_A));

    assert_released(&released);
    assert_eq!(
        released.scope.branch.as_deref(),
        Some(World::branch(items::TICKET_A).as_str())
    );
    assert_release_golden("context_release", &world, &released);
    assert_eq!(
        contexts(&world),
        (
            vec![other_branch.clone()],
            vec![mutation::DOCUMENT.to_owned()]
        )
    );
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    assert!(indexed(&world, &other_branch));
    assert_eq!(world.primary_head(), head);
    assert_eq!(references(&world), kept);
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_3).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn release_of_a_never_created_items_context_is_ok() {
    git_home::isolated();
    let world = World::new();
    let made = world.execute(REQUEST_1, create_ticket(&world, TICKET_NEW));
    assert_ok(&made);

    let released = world.execute(REQUEST_2, release_ticket(&world, TICKET_NEW));

    assert_released(&released);
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert!(!indexed(&world, &World::branch(TICKET_NEW)));
}

#[test]
fn release_with_neither_the_worktree_nor_the_branch_is_already_applied() {
    git_home::isolated();
    let world = World::new();
    let before = image(&world, items::TICKET_A);

    let absent = world.execute(REQUEST_1, release_ticket(&world, items::TICKET_A));

    assert_eq!(
        (absent.outcome, absent.code),
        (Outcome::Noop, ResultCode::AlreadyApplied),
        "{absent:?}"
    );
    assert_eq!(absent.effects, Effects::not_requested());
    assert_eq!(absent.data, None);
    assert_eq!(image(&world, items::TICKET_A), before);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn release_of_a_branch_with_a_commit_is_external_change_and_removes_nothing() {
    git_home::isolated();
    let world = World::new();
    // A save: the context's branch holds the commit of it.
    let token = world.token(items::TICKET_A);
    let saved = world.execute(REQUEST_1, world.save(items::TICKET_A, &token));
    assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");
    assert_eq!(world.branch_commits(items::TICKET_A), 1);
    let before = image(&world, items::TICKET_A);

    let refused = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_refused(&world, REQUEST_2, &refused, ResultCode::ExternalChange);
    assert_eq!(image(&world, items::TICKET_A), before);
}

#[test]
fn release_after_a_proven_publication_is_external_change() {
    git_home::isolated();
    // A tracking ref of the context, under any remote.
    let world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    git2::Repository::open(&world.root)
        .unwrap()
        .reference(
            &format!("refs/remotes/upstream/manyhands/ticket/{}", items::TICKET_A),
            world.primary_head(),
            false,
            "fixture",
        )
        .unwrap();
    let before = image(&world, items::TICKET_A);
    let refused = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));
    assert_refused(&world, REQUEST_2, &refused, ResultCode::ExternalChange);
    assert_eq!(image(&world, items::TICKET_A), before);

    // The push journal of a synchronization that published the context,
    // with no tracking ref.
    let world = World::new();
    select_origin(&world);
    make(&world, REQUEST_1, items::TICKET_A);
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        items::item_id(items::TICKET_A),
    )
    .unwrap();
    let synchronization = OperationId::new();
    assert!(matches!(
        world
            .service
            .reserve_remote_operation(&world.root, synchronization, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    items::index(world.data.path())
        .execute(
            "UPDATE remote_operation_records SET phase='completed',outcome='completed',
                    sync_checkpoint='discovery_pending',completed_step='before_discovery',
                    expected_oid=?2,local_oid=?2,primary_tracking_oid=?2,push_oid=?2,
                    push_advertised_oid=?2,authoritative_kind='published',authoritative_oid=?2,
                    index_pending=1
              WHERE operation_ulid=?1",
            rusqlite::params![
                synchronization.to_string(),
                world.primary_head().to_string()
            ],
        )
        .unwrap();
    assert_eq!(
        evidence(&world, items::TICKET_A),
        RemotePublicationEvidence::ObservedPublished
    );
    let before = image(&world, items::TICKET_A);
    let refused = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));
    assert_refused(&world, REQUEST_2, &refused, ResultCode::ExternalChange);
    assert_eq!(image(&world, items::TICKET_A), before);
}

/// R-43: every re-selection of the remote leaves the publication history
/// unknown for good.
#[test]
fn release_with_an_unknown_publication_history_and_no_tracking_ref_is_ok() {
    git_home::isolated();
    let world = World::new();
    select_origin(&world);
    select(&world, None);
    make(&world, REQUEST_1, items::TICKET_A);
    assert_eq!(
        evidence(&world, items::TICKET_A),
        RemotePublicationEvidence::HistoryUnknown
    );

    let released = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_released(&released);
    assert_eq!(contexts(&world), (vec![], vec![]));
}

#[test]
fn release_of_an_untracked_modified_or_ignored_file_is_worktree_not_clean_and_discards_the_row() {
    git_home::isolated();
    let changes: [(&str, &str); 3] = [
        ("notes.txt", "untracked\n"),
        (".gitignore", "scratch.log\nmodified\n"),
        ("scratch.log", "ignored\n"),
    ];
    for (path, text) in changes {
        let world = World::new();
        items::write(&world.root, ".gitignore", "scratch.log\n");
        items::commit(&world.fixture, &[".gitignore"], items::COMMITTED_AT + 1);
        make(&world, REQUEST_1, items::TICKET_A);
        items::write(&world.worktree(items::TICKET_A), path, text);
        let before = image(&world, items::TICKET_A);
        let rows = journal(&world).len();

        let refused = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

        assert_refused(&world, REQUEST_2, &refused, ResultCode::WorktreeNotClean);
        assert_eq!(image(&world, items::TICKET_A), before, "{path}");
        // The row was begun and discarded.
        let journal = journal(&world);
        assert_eq!(journal.len(), rows + 1, "{path}");
        let (_, action, state, step) = journal.last().unwrap();
        assert_eq!(
            (action.as_str(), state.as_str(), step.as_deref()),
            ("release_context", "completed", Some("rejected")),
            "{path}"
        );
    }
}

#[test]
fn release_with_primary_not_checked_out_is_recovery_required_and_discards_the_row() {
    git_home::isolated();
    let world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    let repository = git2::Repository::open(&world.root).unwrap();
    let head = repository.find_commit(world.primary_head()).unwrap();
    repository.branch("elsewhere", &head, false).unwrap();
    repository.set_head("refs/heads/elsewhere").unwrap();
    let before = image(&world, items::TICKET_A);

    let refused = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_refused(&world, REQUEST_2, &refused, ResultCode::RecoveryRequired);
    assert_eq!(image(&world, items::TICKET_A), before);
    let journal = journal(&world);
    let (_, action, state, step) = journal.last().unwrap();
    assert_eq!(
        (action.as_str(), state.as_str(), step.as_deref()),
        ("release_context", "completed", Some("rejected"))
    );
}

#[test]
fn release_during_an_unfinished_closure_of_the_item_is_recovery_required() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    let preview = world
        .service
        .preview_ticket_closure(PreviewTicketClosureRequest {
            root: world.root.clone(),
            item_id: items::item_id(items::TICKET_A),
            confirmed_identity: None,
        })
        .unwrap();
    let TicketClosurePreflight::Ready(preview) = preview else {
        panic!("the closure is not ready: {preview:?}");
    };
    // The closure stops once it is recorded, before it has done anything.
    fail_once_at(&mut world, FailurePoint::AfterConfirmed);
    let stopped = world.service.close_ticket(
        CloseTicketRequest {
            root: world.root.clone(),
            operation_id: OperationId::new(),
            item_id: items::item_id(items::TICKET_A),
            expected: preview.expectation(),
            approval: None,
            confirmed_identity: None,
        },
        &mut SessionCredentials::new(NoPassphrase),
    );
    assert!(stopped.is_err(), "the closure was not stopped");
    world.reopen();
    let before = image(&world, items::TICKET_A);

    let refused = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_refused(&world, REQUEST_2, &refused, ResultCode::RecoveryRequired);
    assert_eq!(image(&world, items::TICKET_A), before);
}

#[test]
fn release_with_another_pending_row_on_the_root_is_recovery_required() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::BeforeWorktreeCreation);
    let stopped = world.execute(REQUEST_2, create_ticket(&world, TICKET_NEW));
    assert_eq!(stopped.code, ResultCode::InternalError, "{stopped:?}");
    world.reopen();
    let before = image(&world, items::TICKET_A);

    let refused = world.execute(REQUEST_3, release_ticket(&world, items::TICKET_A));

    assert_eq!(refused.code, ResultCode::RecoveryRequired, "{refused:?}");
    assert!(world.record(REQUEST_3).is_none(), "a request record");
    assert_eq!(world.pending_journal_rows(), 1, "the other row alone");
    assert_eq!(image(&world, items::TICKET_A), before);
}

#[test]
fn release_interrupted_before_the_branch_release_other_requests_need_recovery_and_the_same_one_finishes()
 {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::BeforeContextBranchRelease);

    let lost = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    // The worktree is gone and the branch is not.
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );
    assert_eq!(world.pending_journal_rows(), 1);
    world.reopen();
    for (request, mutation) in [
        (REQUEST_3, create_ticket(&world, TICKET_NEW)),
        (
            REQUEST_4,
            release(&world, AuthoringKind::Document, mutation::DOCUMENT),
        ),
    ] {
        let other = world.execute(request, mutation);
        assert_eq!(other.code, ResultCode::RecoveryRequired, "{other:?}");
    }

    let retry = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_released(&retry);
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn release_interrupted_after_the_branch_release_the_same_request_finishes_the_refresh() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::AfterContextBranchRelease);

    let lost = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    assert_eq!(contexts(&world), (vec![], vec![]));
    // The index has not been told.
    assert!(indexed(&world, &World::branch(items::TICKET_A)));
    world.reopen();

    let retry = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_released(&retry);
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn release_whose_refresh_fails_is_discovery_pending_and_resume_completes_it() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::BeforeIndexTransactionCommit);

    let pending = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_eq!(
        (pending.outcome, pending.code),
        (Outcome::Partial, ResultCode::DiscoveryPending),
        "{pending:?}"
    );
    assert_eq!(pending.effects, discovery(DiscoveryEffect::Pending));
    assert_eq!(pending.data, None);
    // The row's step is the branch's, which owes only its hand-off: a
    // resume takes it up, and the result offers one.
    assert_eq!(
        recovery_actions(&pending),
        ["operation.resume", "repo.inspect", "operation.abandon"],
        "{pending:?}"
    );
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert!(indexed(&world, &World::branch(items::TICKET_A)));
    world.reopen();
    // Other requests wait for the row.
    let other = world.execute(REQUEST_3, create_ticket(&world, TICKET_NEW));
    assert_eq!(other.code, ResultCode::RecoveryRequired, "{other:?}");

    let resumed = world.execute(
        REQUEST_4,
        Mutation::OperationResume(OperationResumeInput {
            root: world.root.clone(),
            operation_id: operation_of(&world, REQUEST_2),
        }),
    );

    assert_eq!(
        (resumed.outcome, resumed.code),
        (Outcome::Success, ResultCode::Ok),
        "{resumed:?}"
    );
    assert_eq!(resumed.effects, discovery(DiscoveryEffect::Current));
    assert_eq!(world.pending_journal_rows(), 0);
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    // And the root is free again.
    let made = world.execute(REQUEST_5, create_ticket(&world, TICKET_NEW));
    assert_ok(&made);
}

/// Not a row of the table: the request's own retry finishes the hand-off
/// that a resume would, from the step its row recorded.
#[test]
fn release_whose_refresh_fails_is_finished_by_the_same_request_too() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::BeforeIndexTransactionCommit);
    let pending = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));
    assert_eq!(pending.code, ResultCode::DiscoveryPending, "{pending:?}");
    let operation = pending.operation_id.clone();
    world.reopen();

    let finished = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_released(&finished);
    assert_eq!(finished.operation_id, operation);
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn release_interrupted_before_settlement_the_same_request_is_ok_or_already_applied() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::BeforeRequestSettlement);

    let lost = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Accepted
    );
    world.reopen();

    let retry = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert!(
        matches!(
            (retry.outcome, retry.code),
            (Outcome::Success, ResultCode::Ok) | (Outcome::Noop, ResultCode::AlreadyApplied)
        ),
        "{retry:?}"
    );
    assert_eq!(retry.data, None);
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn release_of_a_branch_left_without_its_worktree_is_ok() {
    git_home::isolated();
    let world = World::new();
    let repository = git2::Repository::open(&world.root).unwrap();
    let head = repository.find_commit(world.primary_head()).unwrap();
    repository
        .branch(&World::branch(items::TICKET_A), &head, false)
        .unwrap();
    assert_eq!(
        contexts(&world),
        (vec![World::branch(items::TICKET_A)], vec![])
    );

    let released = world.execute(REQUEST_1, release_ticket(&world, items::TICKET_A));

    assert_released(&released);
    assert_eq!(contexts(&world), (vec![], vec![]));
}

#[test]
fn release_then_an_edit_of_the_item_makes_its_context_again() {
    git_home::isolated();
    let world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    let released = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));
    assert_released(&released);

    let edit = world.execute(REQUEST_3, edit_ticket(&world, items::TICKET_A));

    assert_ok(&edit);
    assert!(created(&edit), "{edit:?}");
    assert_one_context(&world, items::TICKET_A);
    assert_eq!(world.pending_journal_rows(), 0);
}

/// Not a row of the table (Part 0 review): a context branch with no
/// worktree of its own, checked out in a linked worktree under another
/// name. Step 6 cannot delete it, and since the request touched nothing,
/// its row is discarded and nothing is left pending.
#[test]
fn release_of_a_branch_checked_out_in_another_worktree_is_refused_and_discards_the_row() {
    git_home::isolated();
    let world = World::new();
    let repository = git2::Repository::open(&world.root).unwrap();
    let head = repository.find_commit(world.primary_head()).unwrap();
    let branch = repository
        .branch(&World::branch(items::TICKET_A), &head, false)
        .unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let checkout = elsewhere.path().join("checkout");
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(branch.get()));
    repository
        .worktree("elsewhere", &checkout, Some(&options))
        .unwrap();
    let before = image(&world, items::TICKET_A);

    let refused = world.execute(REQUEST_1, release_ticket(&world, items::TICKET_A));

    assert_refused(&world, REQUEST_1, &refused, ResultCode::ExternalChange);
    let journal = journal(&world);
    let (_, action, state, step) = journal.last().unwrap();
    assert_eq!(
        (action.as_str(), state.as_str(), step.as_deref()),
        ("release_context", "completed", Some("rejected"))
    );
    assert_eq!(image(&world, items::TICKET_A), before);
    assert_eq!(world.branch_tip(items::TICKET_A), Some(head.id()));
    let checked_out = git2::Repository::open(&checkout).unwrap();
    assert_eq!(
        checked_out.head().unwrap().name(),
        Some(format!("refs/heads/{}", World::branch(items::TICKET_A)).as_str())
    );
    // No row of this request stands in the way of what comes next.
    let next = world.execute(REQUEST_2, create_ticket(&world, TICKET_NEW));
    assert_ok(&next);
}

/// R-78: a release records no checkpoint, so one whose answer was lost
/// runs again as new when it is sent again, and releases the context a
/// later Edit made.
#[test]
fn release_lost_before_settlement_then_an_edit_then_the_same_request_releases_the_new_context() {
    git_home::isolated();
    let mut world = World::new();
    make(&world, REQUEST_1, items::TICKET_A);
    fail_once_at(&mut world, FailurePoint::BeforeRequestSettlement);
    let lost = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));
    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    assert_eq!(contexts(&world), (vec![], vec![]));
    world.reopen();
    make(&world, REQUEST_3, items::TICKET_A);

    let again = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));

    assert_released(&again);
    assert_eq!(contexts(&world), (vec![], vec![]));
    assert!(!indexed(&world, &World::branch(items::TICKET_A)));
    assert_eq!(world.pending_journal_rows(), 0);
}

/// R-78: a preparation records no checkpoint either, so one whose answer
/// was lost runs again as new when it is sent again, and makes the context
/// a later release removed.
#[test]
fn prepare_lost_before_settlement_then_a_release_then_the_same_request_makes_the_context_again() {
    git_home::isolated();
    let mut world = World::new();
    fail_once_at(&mut world, FailurePoint::BeforeRequestSettlement);
    let lost = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));
    assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
    assert_one_context(&world, items::TICKET_A);
    world.reopen();
    let released = world.execute(REQUEST_2, release_ticket(&world, items::TICKET_A));
    assert_released(&released);
    assert_eq!(contexts(&world), (vec![], vec![]));

    let again = world.execute(REQUEST_1, edit_ticket(&world, items::TICKET_A));

    assert_ok(&again);
    assert!(created(&again), "{again:?}");
    assert_one_context(&world, items::TICKET_A);
    assert!(indexed(&world, &World::branch(items::TICKET_A)));
    assert_eq!(world.pending_journal_rows(), 0);
}
