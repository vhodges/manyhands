//! `context prepare` through the boundary: the work context (a branch and
//! a worktree) an item is edited in, provisioned for an existing item or
//! a new one. Every row of the design's outcome table is a test here,
//! written before the command was.
//!
//! The last row of that table ends in `context release`, which is a later
//! task's: `prepare_abandoning_the_interrupted_row_and_moving_primary_leaves_a_branch_edit_refuses`
//! covers the part of the row that can be driven until then.

use std::fs;

use manyhands::{
    repository::{
        AuthoringKind, ContextPrepareInput, ContextPrepareIntent, FailurePoint, Mutation,
        MutationDataDto, OperationAbandonInput, OperationId, OperationResumeInput, RequestState,
    },
    results::{DiscoveryEffect, Effects, Envelope, Outcome, ResultCode},
};

use super::support::mutation::Previewed as _;
use super::support::{
    FailOnce, git_home,
    golden::ContractCase,
    items,
    mutation::{
        self, REQUEST_1, REQUEST_2, REQUEST_3, SENTINELS, TICKET_CLOSED, TICKET_NEW, World,
        recovery_actions,
    },
};

const CONTEXT_SCHEMA: &str = "context_mutation.schema.json";

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
    // `context release` removes the stranded branch, after which an Edit
    // answers ok; that part of the row is Task 2's.
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
