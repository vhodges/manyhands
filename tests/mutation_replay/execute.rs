//! `execute` for a request seen for the first time and for one whose
//! record is finished, and how the record of a first call is settled.

use manyhands::{
    canonical::short_code,
    repository::{
        CancellationToken, FailurePoint, LeaseKind, Mutation, MutationDataDto, RelationshipWrite,
        RequestState, TicketCreateInput, TicketDraft, TicketSaveInput, request_store::JournalRow,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, FailureClass, Outcome, ResultCode,
        WriteEffect,
    },
};

#[cfg(unix)]
use crate::support::mutation::owned_path_hook_test_lock;
#[cfg(unix)]
use manyhands::repository::OwnedPathBoundary;

use crate::support::{
    FailOnce, hold_lease_in_child,
    items::{self, TICKET_A, TICKET_ABSENT},
    mutation::{
        BODY, REQUEST_1, REQUEST_2, REQUEST_3, TICKET_CLOSED, TICKET_NEW, TITLE, World,
        execute_cancellable, recovery_actions, ticket_data,
    },
};

#[test]
fn a_new_save_runs_once_and_reports_its_commit() {
    let world = World::new();
    let before = world.branch_commits(TICKET_A);
    assert_eq!(before, 0, "the ticket has no editing context yet");

    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));

    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(envelope.failure_class(), None::<FailureClass>);
    assert_eq!(envelope.command, "ticket save");
    assert_eq!(envelope.request_id.as_deref(), Some(REQUEST_1));
    assert_eq!(envelope.effects.write, WriteEffect::Written);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Current);
    let tip = world.branch_tip(TICKET_A).unwrap();
    assert_eq!(envelope.effects.commit_oid, Some(tip.to_string()));
    assert_eq!(world.branch_commits(TICKET_A), 1, "one checkpoint commit");
    let source = world.worktree_source(TICKET_A);
    assert!(source.contains(TITLE) && source.contains(BODY), "{source}");

    // The scope names the item and its editing context.
    assert_eq!(envelope.scope.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(
        envelope.scope.branch,
        Some(format!("manyhands/ticket/{TICKET_A}"))
    );
    assert_eq!(
        envelope.scope.worktree.as_deref(),
        world.worktree(TICKET_A).to_str()
    );

    // The operation the envelope names is the one the journal completed.
    let operation_id = envelope.operation_id.clone().expect("an operation ID");
    assert!(matches!(
        world.journal_row(&operation_id),
        JournalRow::Final {
            owes_work: false,
            ..
        }
    ));

    // The record is finished and holds the result.
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.attempt, 1);
    assert_eq!(record.command, "ticket save");
    assert_eq!(record.target, TICKET_A);
    assert_eq!(record.operations[0].operation_id.to_string(), operation_id);
    let result = record.result.expect("a stored result");
    assert_eq!(result.outcome, Outcome::Success);
    assert_eq!(result.effects, envelope.effects);
}

fn assert_stopped(
    envelope: &Envelope<MutationDataDto>,
    outcome: Outcome,
    code: ResultCode,
    class: FailureClass,
) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (outcome, code, Some(class))
    );
    assert_eq!(envelope.effects, Effects::not_requested());
}

fn create_input(world: &World, id: &str) -> TicketCreateInput {
    world.create_input(id)
}

#[test]
fn a_create_composes_the_short_code_from_the_configuration_and_the_initials() {
    let world = World::new();

    // No prefix, the default length, and the initials of "Manyhands Test".
    let envelope = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(envelope.command, "ticket create");
    let derived = format!("mt-{}", short_code(TICKET_NEW, 5));
    assert_eq!(ticket_data(&envelope).slug.as_deref(), Some(&*derived));
    assert_eq!(world.slug(TICKET_NEW).as_deref(), Some(&*derived));
    assert_eq!(world.branch_commits(TICKET_NEW), 1);
    assert_eq!(
        envelope.effects.commit_oid,
        world.branch_tip(TICKET_NEW).map(|tip| tip.to_string())
    );

    // The configured prefix and length, and `manyhands.initials`.
    world.commit_configuration("ticket_slug_prefix = \"mh\"\nticket_slug_code_length = 7\n");
    world.set_local_config("manyhands.initials", "QZ");
    let second = "01ARZ3NDEKTSV4RRFFQ69G5FCP";
    let envelope = world.execute(REQUEST_2, world.create(second));
    assert_eq!(envelope.code, ResultCode::Ok);
    let configured = format!("mh-qz-{}", short_code(second, 7));
    assert_eq!(ticket_data(&envelope).slug.as_deref(), Some(&*configured));
    assert_eq!(world.slug(second).as_deref(), Some(&*configured));

    // Initials for one call win and are stored nowhere.
    let before = world.local_config_bytes();
    let third = "01ARZ3NDEKTSV4RRFFQ69G5FCQ";
    let envelope = world.execute(
        REQUEST_3,
        Mutation::TicketCreate(TicketCreateInput {
            initials: Some("Ab3".to_owned()),
            ..create_input(&world, third)
        }),
    );
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(
        world.slug(third),
        Some(format!("mh-ab3-{}", short_code(third, 7)))
    );
    assert_eq!(world.local_config_bytes(), before);
}

#[test]
fn a_create_that_cannot_compose_a_short_code_leaves_nothing() {
    let world = World::new();

    // A name that yields no initials: nothing is transliterated.
    world.set_local_config("user.name", "Ünal Önder");
    let envelope = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::InitialsRequired,
        FailureClass::Blocked,
    );
    assert_eq!(recovery_actions(&envelope), ["repo.identity_set"]);
    assert_eq!(envelope.operation_id, None);
    world.assert_nothing_exists(REQUEST_1, TICKET_NEW);

    // Initials that do not fit are not replaced by derived ones.
    for unfit in ["a", "abcd", "a b", "é1"] {
        let envelope = world.execute(
            REQUEST_1,
            Mutation::TicketCreate(TicketCreateInput {
                initials: Some(unfit.to_owned()),
                ..create_input(&world, TICKET_NEW)
            }),
        );
        assert_eq!(envelope.code, ResultCode::InitialsRequired, "{unfit:?}");
    }
    world.set_local_config("user.name", "Manyhands Test");
    world.set_local_config("manyhands.initials", "toolong");
    let envelope = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(envelope.code, ResultCode::InitialsRequired);
    world.set_local_config("manyhands.initials", "mt");

    // An invalid configured prefix is reported when a slug is composed.
    world.commit_configuration("ticket_slug_prefix = \"Not Valid\"\n");
    let envelope = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::InvalidSlugConfiguration,
        FailureClass::Blocked,
    );
    assert_eq!(recovery_actions(&envelope), ["repo.inspect"]);
    world.assert_nothing_exists(REQUEST_1, TICKET_NEW);
    assert_eq!(world.request_rows(), 0);

    // The repository still works: a save composes no short code.
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_2, world.save(TICKET_A, &token));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
}

#[test]
fn a_relationship_cycle_is_rejected_before_anything_exists() {
    let world = World::new();

    // A ticket naming itself is a cycle of one.
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketCreate(TicketCreateInput {
            deps: vec![TICKET_NEW.to_owned()],
            ..create_input(&world, TICKET_NEW)
        }),
    );
    assert_stopped(
        &envelope,
        Outcome::Error,
        ResultCode::RelationshipCycle,
        FailureClass::Input,
    );
    assert_eq!(ticket_data(&envelope).rejected, [TICKET_NEW]);
    world.assert_nothing_exists(REQUEST_1, TICKET_NEW);

    // A cycle through another ticket: the new one depends on A, then A is
    // saved depending on the new one.
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketCreate(TicketCreateInput {
            deps: vec![TICKET_A.to_owned()],
            ..create_input(&world, TICKET_NEW)
        }),
    );
    assert_eq!(envelope.code, ResultCode::Ok);
    assert!(ticket_data(&envelope).unresolved.is_empty());
    let token = world.token(TICKET_A);
    let cyclic = || {
        Mutation::TicketSave(TicketSaveInput {
            deps: RelationshipWrite::Set(vec![TICKET_NEW.to_owned()]),
            ..world.save_input(TICKET_A, &token)
        })
    };
    let envelope = world.execute(REQUEST_2, cyclic());
    assert_stopped(
        &envelope,
        Outcome::Error,
        ResultCode::RelationshipCycle,
        FailureClass::Input,
    );
    let mut members = [TICKET_A, TICKET_NEW];
    members.sort_unstable();
    assert_eq!(ticket_data(&envelope).rejected, members);
    world.assert_nothing_exists(REQUEST_2, TICKET_A);

    // A parent cycle is the same code.
    let envelope = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            parent: RelationshipWrite::Set(TICKET_A.to_owned()),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(envelope.code, ResultCode::RelationshipCycle);
    world.assert_nothing_exists(REQUEST_2, TICKET_A);

    // A target that is not an ID at all is an invalid relationship.
    let envelope = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            deps: RelationshipWrite::Set(vec!["not an id".to_owned()]),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_stopped(
        &envelope,
        Outcome::Error,
        ResultCode::InvalidRelationship,
        FailureClass::Input,
    );
    world.assert_nothing_exists(REQUEST_2, TICKET_A);
}

#[test]
fn an_unknown_relationship_target_is_accepted_and_listed() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            deps: RelationshipWrite::Set(vec![TICKET_ABSENT.to_owned(), TICKET_CLOSED.to_owned()]),
            parent: RelationshipWrite::Set(TICKET_ABSENT.to_owned()),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(ticket_data(&envelope).unresolved, [TICKET_ABSENT]);
    assert!(ticket_data(&envelope).rejected.is_empty());
    let source = world.worktree_source(TICKET_A);
    assert!(
        source.contains(&format!("parent: {TICKET_ABSENT}")),
        "{source}"
    );
    assert!(
        source.contains(&format!("deps:\n- {TICKET_CLOSED}\n- {TICKET_ABSENT}\n")),
        "{source}"
    );
}

#[test]
fn a_finished_request_is_answered_from_its_record() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Success);
    let record = world.record(REQUEST_1);

    // The same call returns the same envelope and adds no commit.
    let again = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(again, first);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.record(REQUEST_1), record);

    // Another request saves the item again.
    let second_token = world.token(TICKET_A);
    assert_ne!(second_token, token);
    let second = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("Second save", BODY),
            ..world.save_input(TICKET_A, &second_token)
        }),
    );
    assert_eq!(second.outcome, Outcome::Success);
    assert_eq!(world.branch_commits(TICKET_A), 2);

    // The first request is still answered with its own result, and the
    // later save is still on disk.
    let replayed = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(replayed, first);
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert!(world.worktree_source(TICKET_A).contains("Second save"));
    assert_ne!(first.effects.commit_oid, second.effects.commit_oid);
}

#[test]
fn a_finished_request_whose_commit_is_unreachable_requires_recovery() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.effects.checkpoint, CheckpointEffect::Committed);
    let record = world.record(REQUEST_1);

    // The branch is reset behind the commit the request reported.
    let repository = git2::Repository::open(&world.root).unwrap();
    let name = format!("refs/heads/{}", World::branch(TICKET_A));
    repository
        .reference(&name, world.primary_head(), true, "reset")
        .unwrap();
    let source = world.worktree_source(TICKET_A);

    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::RecoveryRequired,
        FailureClass::Blocked,
    );
    assert_eq!(recovery_actions(&envelope), ["repo.inspect"]);
    assert_eq!(world.branch_tip(TICKET_A), Some(world.primary_head()));
    assert_eq!(world.worktree_source(TICKET_A), source);
    assert_eq!(world.record(REQUEST_1), record);
}

#[test]
fn a_finished_request_reused_with_changed_input_is_a_mismatch() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Success);
    let record = world.record(REQUEST_1);
    let tip = world.branch_tip(TICKET_A);
    let source = world.worktree_source(TICKET_A);

    let input = || world.save_input(TICKET_A, &token);
    let changed: Vec<(&str, Mutation)> = vec![
        (
            "body",
            Mutation::TicketSave(TicketSaveInput {
                draft: World::draft(TITLE, "Another body.\n"),
                ..input()
            }),
        ),
        (
            "title",
            Mutation::TicketSave(TicketSaveInput {
                draft: World::draft("Another title", BODY),
                ..input()
            }),
        ),
        (
            "relationship",
            Mutation::TicketSave(TicketSaveInput {
                deps: RelationshipWrite::Set(vec![TICKET_CLOSED.to_owned()]),
                ..input()
            }),
        ),
        (
            "cleared relationship",
            Mutation::TicketSave(TicketSaveInput {
                parent: RelationshipWrite::Clear,
                ..input()
            }),
        ),
        (
            "observation",
            Mutation::TicketSave(world.save_input(TICKET_A, "v1:another")),
        ),
        ("target", world.save(TICKET_CLOSED, &token)),
        ("command", world.create(TICKET_A)),
    ];
    for (what, mutation) in changed {
        let envelope = world.execute(REQUEST_1, mutation);
        assert_eq!(
            (envelope.outcome, envelope.code, envelope.failure_class()),
            (
                Outcome::Error,
                ResultCode::RequestMismatch,
                Some(FailureClass::Input)
            ),
            "{what}"
        );
        assert_eq!(envelope.effects, Effects::not_requested(), "{what}");
        assert!(envelope.recovery.is_empty(), "{what}");
        assert_eq!(world.record(REQUEST_1), record, "{what}");
        assert_eq!(world.branch_tip(TICKET_A), tip, "{what}");
        assert_eq!(world.worktree_source(TICKET_A), source, "{what}");
    }
    assert_eq!(world.request_rows(), 1);
}

fn failing_at(point: FailurePoint) -> World {
    World::with_service(|data| FailOnce::at(point).open_service(data))
}

#[test]
fn an_error_after_the_write_is_partial_and_leaves_the_request_accepted() {
    let world = failing_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));

    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Partial,
            ResultCode::InternalError,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(envelope.effects.write, WriteEffect::Written);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(envelope.effects.commit_oid, None);
    assert_eq!(recovery_actions(&envelope), ["request.retry"]);
    assert_eq!(envelope.recovery[0].arguments["request_id"], REQUEST_1);
    // The file is what the request intended and no commit holds it.
    assert!(world.worktree_source(TICKET_A).contains(TITLE));
    assert_eq!(world.branch_commits(TICKET_A), 0);

    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));
    assert_eq!(record.result, None);
    let operation_id = envelope.operation_id.clone().unwrap();
    assert!(world.journal_row(&operation_id).in_flight());

    // The same request ID with other input is rejected, and the outcome
    // says an earlier attempt of it left an effect.
    let mismatch = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft(TITLE, "Another body.\n"),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(
        (mismatch.outcome, mismatch.code, mismatch.failure_class()),
        (
            Outcome::Partial,
            ResultCode::RequestMismatch,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(mismatch.effects.write, WriteEffect::Written);
    assert_eq!(mismatch.effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(world.record(REQUEST_1), Some(record));
    assert_eq!(world.branch_commits(TICKET_A), 0);
}

#[test]
fn a_failure_before_the_write_is_an_error_and_leaves_the_request_accepted() {
    // The domain marks its effect before this fault point and leaves its
    // row pending, although nothing was written.
    let world = failing_at(FailurePoint::BeforeItemWrite);
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &envelope,
        Outcome::Error,
        ResultCode::InternalError,
        FailureClass::Internal,
    );
    assert_eq!(recovery_actions(&envelope), ["request.retry"]);
    assert!(!world.worktree_source(TICKET_A).contains(TITLE));
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);
    assert!(
        world
            .journal_row(&envelope.operation_id.clone().unwrap())
            .in_flight()
    );

    // Nothing was written, so a changed reuse is an input error.
    let mismatch = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft(TITLE, "Another body.\n"),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_stopped(
        &mismatch,
        Outcome::Error,
        ResultCode::RequestMismatch,
        FailureClass::Input,
    );
}

#[test]
fn a_failure_after_the_checkpoint_is_partial_with_discovery_pending() {
    let world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));

    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Partial,
            ResultCode::DiscoveryPending,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(envelope.effects.write, WriteEffect::Written);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(
        envelope.effects.commit_oid,
        world.branch_tip(TICKET_A).map(|tip| tip.to_string())
    );
    assert_eq!(world.branch_commits(TICKET_A), 1);

    // The action names the operation and takes no argument.
    assert_eq!(recovery_actions(&envelope), ["operation.resume"]);
    assert_eq!(envelope.recovery[0].operation_id, envelope.operation_id);
    assert!(envelope.recovery[0].arguments.is_empty());

    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));
}

#[test]
fn a_rejected_request_leaves_nothing_and_its_id_can_be_used_again() {
    let world = World::new();
    let token = world.token(TICKET_A);

    // A stale token, with content the item does not hold.
    let stale = world.execute(REQUEST_1, world.save(TICKET_A, "v1:stale"));
    assert_stopped(
        &stale,
        Outcome::Blocked,
        ResultCode::ExternalChange,
        FailureClass::Blocked,
    );
    assert!(stale.recovery.is_empty());
    world.assert_nothing_exists(REQUEST_1, TICKET_A);

    // An invalid input.
    let invalid = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("", BODY),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_stopped(
        &invalid,
        Outcome::Error,
        ResultCode::InvalidInput,
        FailureClass::Input,
    );
    world.assert_nothing_exists(REQUEST_1, TICKET_A);

    // The repository lease is held by another process before any write.
    let holder = hold_lease_in_child(&world.root, world.data.path(), LeaseKind::Repository);
    let busy = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &busy,
        Outcome::Error,
        ResultCode::Busy,
        FailureClass::Transient,
    );
    assert_eq!(recovery_actions(&busy), ["request.retry"]);
    holder.release();
    world.assert_nothing_exists(REQUEST_1, TICKET_A);
    assert_eq!(world.request_rows(), 0);

    // A different request runs at once.
    let other = world.execute(REQUEST_2, world.save(TICKET_A, &token));
    assert_eq!(
        (other.outcome, other.code),
        (Outcome::Success, ResultCode::Ok)
    );

    // And the same request ID can be used with corrected input.
    let corrected = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("Corrected", BODY),
            ..world.save_input(TICKET_A, &world.token(TICKET_A))
        }),
    );
    assert_eq!(
        (corrected.outcome, corrected.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn an_uncommitted_edit_on_primary_is_not_saved_over() {
    let world = World::new();
    let path = world.root.join(items::ticket_path(TICKET_A));
    let edited = world
        .primary_source(TICKET_A)
        .replace("Body of", "Edited body of");
    std::fs::write(&path, &edited).unwrap();

    // The caller has seen the edited file: its token is current.
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::WorktreeNotClean,
        FailureClass::Blocked,
    );
    world.assert_nothing_exists(REQUEST_1, TICKET_A);
    assert_eq!(world.primary_source(TICKET_A), edited);
}

#[cfg(unix)]
#[test]
fn a_change_between_the_boundary_s_check_and_the_domain_s_is_an_external_change() {
    let _hook_guard = owned_path_hook_test_lock();
    let world = World::new();
    let token = world.token(TICKET_A);
    let worktree = world.worktree(TICKET_A);
    let file = worktree.join(items::ticket_path(TICKET_A));
    let other = items::ticket_source(TICKET_A, "Changed elsewhere", "");
    let written = other.clone();
    world.service.set_owned_path_hook_for_root_for_testing(
        worktree,
        items::ticket_path(TICKET_A).into(),
        OwnedPathBoundary::Read,
        move || std::fs::write(&file, &written).unwrap(),
    );

    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::ExternalChange,
        FailureClass::Blocked,
    );
    // The domain caught it: an operation ran, and closed its row.
    let operation_id = envelope.operation_id.clone().expect("an operation ID");
    assert_eq!(world.journal_row(&operation_id), JournalRow::Absent);
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);
    // The other content is untouched; only the editing context remains.
    assert_eq!(world.worktree_source(TICKET_A), other);
    assert_eq!(world.branch_commits(TICKET_A), 0);
}

#[test]
fn a_save_that_changes_nothing_is_a_no_op() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let unchanged = || {
        Mutation::TicketSave(TicketSaveInput {
            draft: World::unchanged_draft(),
            ..world.save_input(TICKET_A, &token)
        })
    };
    let envelope = world.execute(REQUEST_1, unchanged());
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_eq!(envelope.effects.write, WriteEffect::Unchanged);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::NotRequested);
    assert_eq!(envelope.effects.commit_oid, None);
    assert_eq!(world.branch_commits(TICKET_A), 0);

    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(world.execute(REQUEST_1, unchanged()), envelope);
}

#[test]
fn a_lifecycle_closed_ticket_is_not_saved_and_a_status_of_closed_is() {
    let world = World::new();
    let token = world.token(TICKET_CLOSED);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_CLOSED, &token));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::TicketClosed,
        FailureClass::Blocked,
    );
    world.assert_nothing_exists(REQUEST_1, TICKET_CLOSED);

    // `status` is text; only the closure metadata closes a ticket.
    let token = world.token(TICKET_A);
    let envelope = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            draft: TicketDraft {
                status: "closed".to_owned(),
                ..World::draft(TITLE, BODY)
            },
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert!(world.worktree_source(TICKET_A).contains("status: closed"));
}

#[test]
fn a_late_caller_asking_for_what_is_already_committed_is_already_applied() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Success);
    let tip = world.branch_tip(TICKET_A);

    // Another request, no record, the token of before the save.
    let late = world.execute(REQUEST_2, world.save(TICKET_A, &token));
    assert_eq!(
        (late.outcome, late.code, late.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_eq!(late.effects.write, WriteEffect::Unchanged);
    assert_eq!(late.effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(late.effects.commit_oid, None);
    assert_eq!(late.operation_id, None);
    assert_eq!(world.branch_tip(TICKET_A), tip);
    assert!(world.record(REQUEST_2).is_none());
    assert_eq!(world.pending_journal_rows(), 0);

    // The same content, with different content asked for: a conflict.
    let other = world.execute(
        REQUEST_3,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("Something else", BODY),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(other.code, ResultCode::ExternalChange);
}

#[test]
fn a_late_caller_asking_for_what_is_written_and_not_committed_is_an_external_change() {
    let world = World::new();
    let token = world.token(TICKET_A);
    // Someone wrote the file to exactly what the request intends and did
    // not commit it.
    let path = world.root.join(items::ticket_path(TICKET_A));
    let written = world
        .primary_source(TICKET_A)
        .replace("title: Open ticket", "title: Retitled");
    std::fs::write(&path, &written).unwrap();

    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("Retitled", "Body of Open ticket.\n"),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::ExternalChange,
        FailureClass::Blocked,
    );
    world.assert_nothing_exists(REQUEST_1, TICKET_A);
    assert_eq!(world.primary_source(TICKET_A), written);
}

#[test]
fn a_short_code_is_unchanged_by_an_identity_change_and_by_a_prefix_change() {
    let world = World::new();
    let created = world.execute(REQUEST_1, world.create(TICKET_NEW));
    let slug = ticket_data(&created).slug.clone().expect("a short code");
    assert_eq!(world.slug(TICKET_NEW), Some(slug.clone()));

    world.set_local_config("user.name", "Zoe Young");
    let token = world.token(TICKET_NEW);
    let saved = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("After the identity changed", BODY),
            ..world.save_input(TICKET_NEW, &token)
        }),
    );
    assert_eq!(
        (saved.outcome, saved.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(ticket_data(&saved).slug, Some(slug.clone()));
    assert_eq!(world.slug(TICKET_NEW), Some(slug.clone()));

    world.commit_configuration("ticket_slug_prefix = \"zz\"\nticket_slug_code_length = 8\n");
    let token = world.token(TICKET_NEW);
    let saved = world.execute(
        REQUEST_3,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("After the prefix changed", BODY),
            ..world.save_input(TICKET_NEW, &token)
        }),
    );
    assert_eq!(
        (saved.outcome, saved.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(ticket_data(&saved).slug, Some(slug.clone()));
    assert_eq!(world.slug(TICKET_NEW), Some(slug));
    assert_eq!(world.branch_commits(TICKET_NEW), 3);
}

#[test]
fn a_request_cancelled_before_it_is_accepted_does_nothing() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let envelope = execute_cancellable(
        &world.service,
        REQUEST_1,
        world.save(TICKET_A, &token),
        &cancel,
    );
    assert_stopped(
        &envelope,
        Outcome::Cancelled,
        ResultCode::Cancelled,
        FailureClass::Cancelled,
    );
    world.assert_nothing_exists(REQUEST_1, TICKET_A);
}

#[test]
fn a_repository_that_is_not_registered_is_reported_with_its_rebuild() {
    let world = World::new();
    let elsewhere = crate::support::born_repository();
    let root = elsewhere.root.canonicalize().unwrap();
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            root: root.clone(),
            ..world.save_input(TICKET_A, "v1:any")
        }),
    );
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::RepositoryNotRegistered,
        FailureClass::Blocked,
    );
    assert_eq!(recovery_actions(&envelope), ["index.rebuild"]);
    assert_eq!(
        envelope.recovery[0].arguments["root"],
        root.to_str().unwrap()
    );
    assert_eq!(world.request_rows(), 0);
}

#[test]
fn a_commit_the_domain_names_and_did_not_make_is_not_reported() {
    // With the refresh mark failing, a save that changes nothing comes
    // back from the domain naming the head it found as its checkpoint.
    let world = failing_at(FailurePoint::BeforeRegistryWrite);
    let token = world.token(TICKET_A);
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::unchanged_draft(),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(envelope.effects.commit_oid, None);
    assert_eq!(world.branch_commits(TICKET_A), 0);
}

fn another_body(world: &World, token: &str) -> Mutation {
    Mutation::TicketSave(TicketSaveInput {
        draft: World::draft(TITLE, "Another body.\n"),
        ..world.save_input(TICKET_A, token)
    })
}

#[test]
fn a_changed_reuse_after_an_unsettled_commit_is_partial_with_that_commit() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Success);
    // The process died after the domain committed and completed its row and
    // before the record was settled. No fault point reaches that yet, so
    // the record is put back as accepted through the store.
    items::index(world.data.path())
        .execute(
            "UPDATE request_records SET state = 'accepted', outcome = NULL, code = NULL,
                 effect_write = NULL, effect_checkpoint = NULL, effect_discovery = NULL,
                 effect_publication = NULL, effect_integration = NULL, effect_cleanup = NULL,
                 commit_oid = NULL, result_data = NULL, finished_at = NULL",
            [],
        )
        .unwrap();
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Accepted);
    assert!(
        !world
            .journal_row(&first.operation_id.clone().unwrap())
            .in_flight()
    );

    let mismatch = world.execute(REQUEST_1, another_body(&world, &token));
    assert_eq!(
        (mismatch.outcome, mismatch.code, mismatch.failure_class()),
        (
            Outcome::Partial,
            ResultCode::RequestMismatch,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(mismatch.effects.write, WriteEffect::Written);
    assert_eq!(mismatch.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(mismatch.effects.commit_oid, first.effects.commit_oid);
    assert_eq!(world.record(REQUEST_1), Some(record));
    assert_eq!(world.branch_commits(TICKET_A), 1);
}

#[test]
fn a_changed_reuse_of_a_request_that_made_no_context_reports_no_write() {
    // The domain marks its effect before this fault point: the row stays
    // pending, and there is no branch and no worktree.
    let world = failing_at(FailurePoint::BeforeContextBranchCreation);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &first,
        Outcome::Error,
        ResultCode::InternalError,
        FailureClass::Internal,
    );
    assert!(!world.worktree(TICKET_A).exists());
    assert!(
        world
            .journal_row(&first.operation_id.clone().unwrap())
            .in_flight()
    );
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);

    let mismatch = world.execute(REQUEST_1, another_body(&world, &token));
    assert_stopped(
        &mismatch,
        Outcome::Error,
        ResultCode::RequestMismatch,
        FailureClass::Input,
    );
    assert_eq!(world.record(REQUEST_1), Some(record));
}

#[test]
fn a_changed_reuse_after_a_commit_with_discovery_pending_reports_the_commit() {
    let world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::DiscoveryPending);

    let mismatch = world.execute(REQUEST_1, another_body(&world, &token));
    assert_eq!(
        (mismatch.outcome, mismatch.code),
        (Outcome::Partial, ResultCode::RequestMismatch)
    );
    assert_eq!(mismatch.effects.write, WriteEffect::Written);
    assert_eq!(mismatch.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(mismatch.effects.commit_oid, first.effects.commit_oid);
    assert!(mismatch.effects.commit_oid.is_some());
}

#[test]
fn an_error_after_the_commit_is_partial_with_that_commit() {
    let mut world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Success);

    // The file in the context already holds what the next request intends,
    // uncommitted, and the caller has seen it.
    let file = world.worktree(TICKET_A).join(items::ticket_path(TICKET_A));
    let written = world
        .worktree_source(TICKET_A)
        .replace(TITLE, "Retitled by hand");
    std::fs::write(&file, &written).unwrap();
    let token = world.token(TICKET_A);

    // So the domain skips the write, commits, and then fails to record
    // the checkpoint step.
    world.service = FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
        .open_service(world.data.path());
    let envelope = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft("Retitled by hand", BODY),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Partial,
            ResultCode::InternalError,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert_eq!(envelope.effects.write, WriteEffect::Written);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(
        envelope.effects.commit_oid,
        world.branch_tip(TICKET_A).map(|tip| tip.to_string())
    );
    assert_ne!(envelope.effects.commit_oid, first.effects.commit_oid);
    assert_eq!(recovery_actions(&envelope), ["request.retry"]);
    let record = world.record(REQUEST_2).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));
    assert_eq!(world.worktree_source(TICKET_A), written);
}

const NO_IDENTITY_HOME: &str = "MANYHANDS_NO_IDENTITY_HOME";

/// Runs in a process of its own: where libgit2 looks for configuration is
/// process-wide, and this points every level but the repository's at an
/// empty directory so that no identity is found.
#[test]
fn a_save_with_no_identity_is_blocked_and_its_request_id_is_free() {
    let Some(home) = std::env::var_os(NO_IDENTITY_HOME) else {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "execute::a_save_with_no_identity_is_blocked_and_its_request_id_is_free",
                "--nocapture",
            ])
            .env(NO_IDENTITY_HOME, home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "the child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };
    for level in [
        git2::ConfigLevel::System,
        git2::ConfigLevel::Global,
        git2::ConfigLevel::XDG,
        git2::ConfigLevel::ProgramData,
    ] {
        // This exact-test child runs nothing else, so no other use of
        // libgit2 overlaps the change.
        unsafe { git2::opts::set_search_path(level, &home).unwrap() };
    }
    let world = World::new();
    let token = world.token(TICKET_A);
    let mut local = git2::Repository::open(&world.root)
        .unwrap()
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    local.remove("user.name").unwrap();
    local.remove("user.email").unwrap();

    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_stopped(
        &envelope,
        Outcome::Blocked,
        ResultCode::IdentityRequired,
        FailureClass::Blocked,
    );
    assert_eq!(recovery_actions(&envelope), ["repo.identity_set"]);
    // The domain ran and completed its row: nothing is in flight, so the
    // record is deleted. The editing context it made stays.
    let operation_id = envelope.operation_id.clone().expect("an operation ID");
    assert!(matches!(
        world.journal_row(&operation_id),
        JournalRow::Final {
            owes_work: false,
            ..
        }
    ));
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);
    assert!(world.worktree(TICKET_A).is_dir());
    assert_eq!(world.branch_commits(TICKET_A), 0);
    assert!(!world.worktree_source(TICKET_A).contains(TITLE));

    // With an identity the same request ID runs.
    world.set_local_config("user.name", "Manyhands Test");
    world.set_local_config("user.email", "manyhands-test@example.invalid");
    let again = world.execute(REQUEST_1, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(
        (again.outcome, again.code),
        (Outcome::Success, ResultCode::Ok)
    );
}
