//! `document create`, `document save` and `document move`.

use manyhands::{
    repository::{
        DocumentMoveInput, DocumentSaveInput, FailurePoint, Mutation, MutationDataDto, RequestState,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, FailureClass, Outcome, ResultCode,
        WriteEffect,
    },
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{
        BODY, DOCUMENT, DOCUMENT_MOVED_PATH, DOCUMENT_NEW, DOCUMENT_NEW_PATH, DOCUMENT_PATH,
        REQUEST_1, REQUEST_2, REQUEST_3, TITLE, World, document_data, recovery_actions,
    },
};

fn assert_committed(world: &World, id: &str, envelope: &Envelope<MutationDataDto>, what: &str) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None),
        "{what}"
    );
    assert_eq!(envelope.effects.write, WriteEffect::Written, "{what}");
    assert_eq!(
        envelope.effects.checkpoint,
        CheckpointEffect::Committed,
        "{what}"
    );
    assert_eq!(
        envelope.effects.discovery,
        DiscoveryEffect::Current,
        "{what}"
    );
    assert_eq!(
        envelope.effects.commit_oid,
        world.branch_tip(id).map(|tip| tip.to_string()),
        "{what}"
    );
    assert_eq!(world.pending_journal_rows(), 0, "{what}");
}

fn assert_rejected(
    envelope: &Envelope<MutationDataDto>,
    outcome: Outcome,
    code: ResultCode,
    class: FailureClass,
    what: &str,
) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (outcome, code, Some(class)),
        "{what}"
    );
    assert_eq!(envelope.effects, Effects::not_requested(), "{what}");
    assert_eq!(envelope.operation_id, None, "{what}");
    assert!(recovery_actions(envelope).is_empty(), "{what}");
}

/// A move of the fixture's document to `DOCUMENT_MOVED_PATH`, with the
/// tokens of the document and of the destination as they are now.
fn move_now(world: &World) -> Mutation {
    world.document_move(
        DOCUMENT,
        &world.token(DOCUMENT),
        DOCUMENT_MOVED_PATH,
        &world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
    )
}

#[test]
fn a_document_is_created_at_its_path() {
    let world = World::new();
    let envelope = world.execute(
        REQUEST_1,
        world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
    );

    assert_committed(&world, DOCUMENT_NEW, &envelope, "the create");
    assert_eq!(envelope.command, "document create");
    assert_eq!(envelope.scope.item_id.as_deref(), Some(DOCUMENT_NEW));
    assert_eq!(
        envelope.scope.branch,
        Some(World::document_branch(DOCUMENT_NEW))
    );
    assert_eq!(
        envelope.scope.worktree.as_deref(),
        world.worktree(DOCUMENT_NEW).to_str()
    );
    let data = document_data(&envelope);
    assert_eq!(
        (data.id.as_str(), data.path.as_deref()),
        (DOCUMENT_NEW, Some(DOCUMENT_NEW_PATH))
    );
    assert_eq!(world.branch_commits(DOCUMENT_NEW), 1);
    let source = world
        .context_source(DOCUMENT_NEW, DOCUMENT_NEW_PATH)
        .expect("the document's file");
    assert!(source.contains(TITLE) && source.contains(BODY), "{source}");
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.command, "document create");
    assert_eq!(record.target, DOCUMENT_NEW);

    // Another request for the same document: what it asks for holds.
    let again = world.execute(
        REQUEST_2,
        world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
    );
    assert_eq!(
        (again.outcome, again.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(again.effects.commit_oid, None);
    assert!(world.record(REQUEST_2).is_none());
    assert_eq!(world.branch_commits(DOCUMENT_NEW), 1);
}

#[test]
fn a_create_with_an_id_or_a_path_that_is_taken_is_occupied_path() {
    let world = World::new();
    let primary = world.primary_head();
    let cases = [
        (
            "a document's ID, at another path",
            world.document_create(DOCUMENT, DOCUMENT_NEW_PATH),
            DOCUMENT,
        ),
        (
            "a document's ID, at its path, with other content",
            world.document_create(DOCUMENT, DOCUMENT_PATH),
            DOCUMENT,
        ),
        (
            "a ticket's ID",
            world.document_create(TICKET_A, DOCUMENT_NEW_PATH),
            TICKET_A,
        ),
        (
            "a new ID, at a path a document is at",
            world.document_create(DOCUMENT_NEW, DOCUMENT_PATH),
            DOCUMENT_NEW,
        ),
    ];
    for (what, mutation, id) in cases {
        let envelope = world.execute(REQUEST_1, mutation);
        assert_rejected(
            &envelope,
            Outcome::Error,
            ResultCode::OccupiedPath,
            FailureClass::Input,
            what,
        );
        world.assert_nothing_exists(REQUEST_1, id);
    }
    assert_eq!(world.primary_head(), primary);
    assert_eq!(
        world.primary_source(TICKET_A),
        items::ticket_source(TICKET_A, "Open ticket", "")
    );
}

#[test]
fn a_create_at_a_path_no_document_can_be_at_is_an_invalid_path() {
    let world = World::new();
    for path in [
        "notes/new.md",
        "docs/new.txt",
        "docs/../new.md",
        "/docs/new.md",
        "docs",
    ] {
        let envelope = world.execute(REQUEST_1, world.document_create(DOCUMENT_NEW, path));
        assert_rejected(
            &envelope,
            Outcome::Error,
            ResultCode::InvalidPath,
            FailureClass::Input,
            path,
        );
        world.assert_nothing_exists(REQUEST_1, DOCUMENT_NEW);
    }
}

#[test]
fn a_save_or_a_move_without_the_source_observation_is_invalid_input() {
    let world = World::new();
    let primary = world.primary_source_at(DOCUMENT_PATH);
    let token = world.token(DOCUMENT);
    let absent = world.observe(DOCUMENT, DOCUMENT_MOVED_PATH);
    let cases = [
        (
            "a save",
            Mutation::DocumentSave(DocumentSaveInput {
                observation: None,
                ..world.document_save_input(DOCUMENT, &token)
            }),
        ),
        (
            "a move",
            Mutation::DocumentMove(DocumentMoveInput {
                observation: None,
                ..world.document_move_input(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent)
            }),
        ),
    ];
    for (what, mutation) in cases {
        let envelope = world.execute(REQUEST_1, mutation);
        assert_rejected(
            &envelope,
            Outcome::Error,
            ResultCode::InvalidInput,
            FailureClass::Input,
            what,
        );
        // Before any write, removal or checkpoint.
        world.assert_nothing_exists(REQUEST_1, DOCUMENT);
        assert_eq!(world.primary_source_at(DOCUMENT_PATH), primary, "{what}");
        assert!(!world.root.join(DOCUMENT_MOVED_PATH).exists(), "{what}");
        assert_eq!(world.request_rows(), 0, "{what}");
    }
}

#[test]
fn a_document_is_saved_where_it_is() {
    let world = World::new();
    let token = world.token(DOCUMENT);
    let envelope = world.execute(REQUEST_1, world.document_save(DOCUMENT, &token));

    assert_committed(&world, DOCUMENT, &envelope, "the save");
    assert_eq!(envelope.command, "document save");
    assert_eq!(
        document_data(&envelope).path.as_deref(),
        Some(DOCUMENT_PATH)
    );
    assert_eq!(world.branch_commits(DOCUMENT), 1);
    let source = world.context_source(DOCUMENT, DOCUMENT_PATH).unwrap();
    assert!(source.contains(TITLE) && source.contains(BODY), "{source}");
    // Primary is as it was.
    assert_eq!(
        world.primary_source_at(DOCUMENT_PATH),
        items::document_source(DOCUMENT, "Guide", "")
    );

    // The first token is stale now. The same content under another
    // request is what already holds; other content is an external change.
    let late = world.execute(REQUEST_2, world.document_save(DOCUMENT, &token));
    assert_eq!(
        (late.outcome, late.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    let other = world.execute(
        REQUEST_3,
        Mutation::DocumentSave(DocumentSaveInput {
            draft: manyhands::repository::DocumentDraft {
                title: "Another title".to_owned(),
                body: BODY.to_owned(),
            },
            ..world.document_save_input(DOCUMENT, &token)
        }),
    );
    assert_rejected(
        &other,
        Outcome::Blocked,
        ResultCode::ExternalChange,
        FailureClass::Blocked,
        "a stale token",
    );
    assert_eq!(world.branch_commits(DOCUMENT), 1);
    assert_eq!(world.request_rows(), 1);

    // A save that changes nothing is a no-op with no commit.
    let unchanged = world.execute(
        REQUEST_3,
        world.document_save(DOCUMENT, &world.token(DOCUMENT)),
    );
    assert_eq!(
        (unchanged.outcome, unchanged.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(unchanged.effects.commit_oid, None);
    assert_eq!(world.branch_commits(DOCUMENT), 1);
}

#[test]
fn an_uncommitted_edit_of_a_document_on_primary_is_not_saved_over() {
    let world = World::new();
    std::fs::write(
        world.root.join(DOCUMENT_PATH),
        items::document_source(DOCUMENT, "Edited by hand", ""),
    )
    .unwrap();
    // The caller has seen the edited file: its token is current.
    let token = world.token(DOCUMENT);
    let envelope = world.execute(REQUEST_1, world.document_save(DOCUMENT, &token));
    assert_rejected(
        &envelope,
        Outcome::Blocked,
        ResultCode::WorktreeNotClean,
        FailureClass::Blocked,
        "the save",
    );
    world.assert_nothing_exists(REQUEST_1, DOCUMENT);
}

#[test]
fn a_move_commits_both_paths_in_one_commit() {
    let world = World::new();
    let before = world.primary_source_at(DOCUMENT_PATH);
    let envelope = world.execute(REQUEST_1, move_now(&world));

    assert_committed(&world, DOCUMENT, &envelope, "the move");
    assert_eq!(envelope.command, "document move");
    assert_eq!(
        document_data(&envelope).path.as_deref(),
        Some(DOCUMENT_MOVED_PATH)
    );
    // One commit: the file is at the destination and no longer at the
    // path it was moved from.
    assert_eq!(world.branch_commits(DOCUMENT), 1);
    let tip = world.branch_tip(DOCUMENT).unwrap();
    assert_eq!(world.documents_at(tip), [DOCUMENT_MOVED_PATH]);
    assert_eq!(world.documents_at(world.primary_head()), [DOCUMENT_PATH]);
    assert_eq!(world.context_source(DOCUMENT, DOCUMENT_PATH), None);
    // The document is the one it was: its ID, title and body.
    let moved = world
        .context_source(DOCUMENT, DOCUMENT_MOVED_PATH)
        .expect("the document at its destination");
    assert!(
        moved.contains(&format!("id: {DOCUMENT}")) && moved.contains("title: Guide"),
        "{moved}"
    );
    assert!(moved.ends_with("---\n# Guide\n"), "{moved}");
    assert_eq!(world.primary_source_at(DOCUMENT_PATH), before);

    // Another request for the same move: the document is there already.
    let again = world.execute(
        REQUEST_2,
        world.document_move(DOCUMENT, "v1:stale", DOCUMENT_MOVED_PATH, "v1:stale"),
    );
    assert_eq!(
        (again.outcome, again.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(world.branch_commits(DOCUMENT), 1);
}

#[test]
fn a_move_checks_the_token_of_each_path() {
    let world = World::new();
    let token = world.token(DOCUMENT);
    let absent = world.observe(DOCUMENT, DOCUMENT_MOVED_PATH);
    let source = world.primary_source_at(DOCUMENT_PATH);
    let refused = |what: &str, mutation: Mutation, outcome, code, class| {
        let envelope = world.execute(REQUEST_1, mutation);
        assert_rejected(&envelope, outcome, code, class, what);
        world.assert_nothing_exists(REQUEST_1, DOCUMENT);
        assert_eq!(world.primary_source_at(DOCUMENT_PATH), source, "{what}");
    };
    let external = (
        Outcome::Blocked,
        ResultCode::ExternalChange,
        FailureClass::Blocked,
    );

    refused(
        "a stale source token",
        world.document_move(DOCUMENT, "v1:stale", DOCUMENT_MOVED_PATH, &absent),
        external.0,
        external.1,
        external.2,
    );
    refused(
        "a stale destination token",
        world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, "v1:stale"),
        external.0,
        external.1,
        external.2,
    );
    // The absent token of another path is not this destination's.
    refused(
        "the absent token of another path",
        world.document_move(
            DOCUMENT,
            &token,
            DOCUMENT_MOVED_PATH,
            &world.observe(DOCUMENT, "docs/moved/other.md"),
        ),
        external.0,
        external.1,
        external.2,
    );

    // Someone puts a file at the destination after the caller observed
    // it absent: the caller's token of the destination is stale.
    items::write(&world.root, DOCUMENT_MOVED_PATH, "Something else.\n");
    refused(
        "a destination that was absent and is not",
        world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent),
        external.0,
        external.1,
        external.2,
    );
    // The caller has seen that file: the destination is occupied.
    refused(
        "an occupied destination",
        world.document_move(
            DOCUMENT,
            &token,
            DOCUMENT_MOVED_PATH,
            &world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
        ),
        Outcome::Error,
        ResultCode::OccupiedPath,
        FailureClass::Input,
    );
    assert_eq!(
        world.primary_source_at(DOCUMENT_MOVED_PATH),
        "Something else.\n"
    );
    assert_eq!(world.request_rows(), 0);
}

/// A move interrupted before it wrote anything: its record is accepted,
/// its operation's row is pending and the document's context exists.
fn interrupted_move() -> (World, Mutation) {
    let mut world =
        World::with_service(|data| FailOnce::at(FailurePoint::BeforeItemWrite).open_service(data));
    let (token, absent) = (
        world.token(DOCUMENT),
        world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
    );
    let request =
        |world: &World| world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent);
    let first = world.execute(REQUEST_1, request(&world));
    assert_eq!(
        (first.outcome, first.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(recovery_actions(&first), ["request.retry"]);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    assert!(world.worktree(DOCUMENT).is_dir());
    world.reopen();
    let retry = request(&world);
    (world, retry)
}

#[test]
fn a_foreign_commit_to_either_path_of_a_move_in_flight_is_an_external_change() {
    for (what, path) in [
        ("the path it moves from", DOCUMENT_PATH),
        ("its destination", DOCUMENT_MOVED_PATH),
    ] {
        let (world, retry) = interrupted_move();
        let foreign = world.commit_in_context(
            DOCUMENT,
            path,
            &items::document_source(DOCUMENT, "Someone else's", ""),
        );

        let envelope = world.execute(REQUEST_1, retry);
        assert_eq!(
            (envelope.outcome, envelope.code, envelope.failure_class()),
            (
                Outcome::Blocked,
                ResultCode::ExternalChange,
                Some(FailureClass::Blocked)
            ),
            "{what}"
        );
        assert_eq!(envelope.effects, Effects::not_requested(), "{what}");
        // Read the item again and use a new request: no action is offered.
        assert!(recovery_actions(&envelope).is_empty(), "{what}");
        // Nothing was written, and the other commit is the branch's tip.
        assert_eq!(world.branch_tip(DOCUMENT), Some(foreign), "{what}");
        // The operation's row is still pending, so the record stays
        // accepted and names no result: this is open case 1.
        let record = world.record(REQUEST_1).expect("the record stays");
        assert_eq!(record.state, RequestState::Accepted, "{what}");
        assert_eq!(record.result, None, "{what}");
        assert_eq!(world.pending_journal_rows(), 1, "{what}");
        assert_eq!(world.request_rows(), 1, "{what}");
        assert_eq!(
            world.context_source(DOCUMENT, path),
            Some(items::document_source(DOCUMENT, "Someone else's", "")),
            "{what}"
        );
    }
}

#[test]
fn an_unrelated_commit_does_not_stop_a_move_in_flight() {
    let (world, retry) = interrupted_move();
    world.commit_in_context(DOCUMENT, "docs/unrelated.md", "Unrelated.\n");

    let envelope = world.execute(REQUEST_1, retry);
    assert_committed(&world, DOCUMENT, &envelope, "the retry");
    assert_eq!(world.branch_commits(DOCUMENT), 2);
    let tip = world.branch_tip(DOCUMENT).unwrap();
    assert_eq!(
        world.documents_at(tip),
        [DOCUMENT_MOVED_PATH, "docs/unrelated.md"]
    );
}

#[test]
fn an_edit_made_under_an_interrupted_move_is_not_committed_as_its_own() {
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let (token, absent) = (
        world.token(DOCUMENT),
        world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
    );
    let first = world.execute(
        REQUEST_1,
        world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent),
    );
    assert_eq!(first.outcome, Outcome::Partial);
    assert_eq!(first.effects.checkpoint, CheckpointEffect::Pending);

    // Someone edits the file at the destination and does not commit.
    let edited = items::document_source(DOCUMENT, "Edited by hand", "");
    std::fs::write(world.worktree(DOCUMENT).join(DOCUMENT_MOVED_PATH), &edited).unwrap();

    world.reopen();
    let retry = world.execute(
        REQUEST_1,
        world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent),
    );
    // The file at the destination is no longer what the request intends,
    // so no write is reported as the request's.
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::ExternalChange,
            Some(FailureClass::Blocked)
        )
    );
    assert_eq!(retry.effects, Effects::not_requested());
    assert!(recovery_actions(&retry).is_empty());
    // The row is pending, having recorded the steps that made the
    // context: the record stays accepted.
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);
    assert_eq!(record.result, None);
    assert_eq!(world.pending_journal_rows(), 1);
    assert_eq!(world.branch_commits(DOCUMENT), 0, "nothing was committed");
    // The file it moved from is where it was.
    assert!(world.context_source(DOCUMENT, DOCUMENT_PATH).is_some());
    assert_eq!(
        world.context_source(DOCUMENT, DOCUMENT_MOVED_PATH),
        Some(edited)
    );
}

#[test]
fn a_create_s_own_file_in_a_context_the_index_has_not_seen_is_not_an_occupied_path() {
    // An earlier attempt wrote the file and did not commit; one that
    // committed and did not reach the index. Either way the index does not
    // hold the document, and another request for the same create finds
    // the file in the context the attempt left.
    for (point, outcome, code) in [
        (
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
            Outcome::Blocked,
            ResultCode::ExternalChange,
        ),
        (
            FailurePoint::BeforeIndexTransactionCommit,
            Outcome::Noop,
            ResultCode::AlreadyApplied,
        ),
    ] {
        let what = format!("{point:?}");
        let world = World::with_service(|data| FailOnce::at(point).open_service(data));
        let first = world.execute(
            REQUEST_1,
            world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
        );
        assert_eq!(first.outcome, Outcome::Partial, "{what}");
        assert!(
            world
                .service
                .show_item(&world.resolved(), &items::item_id(DOCUMENT_NEW))
                .is_err(),
            "{what}: the index does not hold the document"
        );
        let tip = world.branch_tip(DOCUMENT_NEW);

        let again = world.execute(
            REQUEST_2,
            world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
        );
        assert_eq!((again.outcome, again.code), (outcome, code), "{what}");
        assert_eq!(again.effects.commit_oid, None, "{what}");
        assert_eq!(again.operation_id, None, "{what}");
        assert!(world.record(REQUEST_2).is_none(), "{what}");
        assert_eq!(world.branch_tip(DOCUMENT_NEW), tip, "{what}");

        // Other content at the path, in that context, is in the way.
        let other = world.execute(
            REQUEST_3,
            Mutation::DocumentCreate(manyhands::repository::DocumentCreateInput {
                root: world.root.clone(),
                id: items::item_id(DOCUMENT_NEW),
                path: DOCUMENT_NEW_PATH.into(),
                draft: manyhands::repository::DocumentDraft {
                    title: "Another title".to_owned(),
                    body: BODY.to_owned(),
                },
            }),
        );
        assert_eq!(other.code, ResultCode::OccupiedPath, "{what}");
    }
}

#[test]
fn a_move_whose_removal_was_never_committed_is_not_answered_as_a_no_op() {
    // The move writes the destination, removes the file it moved from and
    // dies before its commit. Then its record is lost.
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeCheckpointCommit).open_service(data)
    });
    let (token, absent) = (
        world.token(DOCUMENT),
        world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
    );
    let request =
        |world: &World| world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent);
    let first = world.execute(REQUEST_1, request(&world));
    assert_eq!(first.outcome, Outcome::Partial);
    assert_eq!(world.branch_commits(DOCUMENT), 0);
    world.lose_cache();
    world.rebuild();

    // Sent again it is a new request: written and not committed.
    let retry = world.execute(REQUEST_1, request(&world));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Blocked, ResultCode::ExternalChange)
    );

    // The caller reads the document again and saves it. A save commits
    // the file where it is, and not the removal of the file it was moved
    // from: the branch now holds the document at both paths.
    let saved = world.execute(
        REQUEST_2,
        world.document_save(DOCUMENT, &world.token(DOCUMENT)),
    );
    assert_eq!(saved.code, ResultCode::Ok);
    let tip = world.branch_tip(DOCUMENT).unwrap();
    assert_eq!(
        world.documents_at(tip),
        [DOCUMENT_PATH, DOCUMENT_MOVED_PATH]
    );

    // The move's end state does not hold, and no move is told it does:
    // not the first one sent again, and not a new one with current tokens.
    for (what, mutation) in [
        ("the same move", request(&world)),
        (
            "a move with current tokens",
            world.document_move(
                DOCUMENT,
                &world.token(DOCUMENT),
                DOCUMENT_MOVED_PATH,
                &world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
            ),
        ),
    ] {
        let envelope = world.execute(REQUEST_3, mutation);
        assert_rejected(
            &envelope,
            Outcome::Blocked,
            ResultCode::ExternalChange,
            FailureClass::Blocked,
            what,
        );
        assert!(world.record(REQUEST_3).is_none(), "{what}");
        assert_eq!(world.branch_tip(DOCUMENT), Some(tip), "{what}");
    }
}

/// Commits the removal of `path` on the branch checked out in the
/// document's context, as `git rm --cached` and a commit would: the
/// worktree is left as it is.
fn commit_removal_in_context(world: &World, id: &str, path: &str) -> git2::Oid {
    let repository = git2::Repository::open(world.worktree(id)).unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    let mut index = repository.index().unwrap();
    index.read_tree(&head.tree().unwrap()).unwrap();
    index.remove(std::path::Path::new(path), 0).unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    index.write().unwrap();
    let signature = git2::Signature::new(
        "Someone Else",
        "someone-else@example.invalid",
        &git2::Time::new(items::COMMITTED_AT + 60, 0),
    )
    .unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "by hand",
            &tree,
            &[&head],
        )
        .unwrap()
}

#[test]
fn a_move_whose_removal_someone_else_committed_reports_its_own_commit() {
    // The move wrote the destination and removed the file it moved from,
    // and died before its commit.
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeCheckpointCommit).open_service(data)
    });
    let (token, absent) = (
        world.token(DOCUMENT),
        world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
    );
    let request =
        |world: &World| world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent);
    let first = world.execute(REQUEST_1, request(&world));
    assert_eq!(first.outcome, Outcome::Partial);
    // Someone commits the removal, and only that.
    let removal = commit_removal_in_context(&world, DOCUMENT, DOCUMENT_PATH);
    assert!(world.documents_at(removal).is_empty());

    world.reopen();
    let retry = world.execute(REQUEST_1, request(&world));
    // The retry commits the destination: that commit is the request's.
    assert_committed(&world, DOCUMENT, &retry, "the retry");
    let tip = world.branch_tip(DOCUMENT).unwrap();
    assert_ne!(tip, removal);
    assert_eq!(world.documents_at(tip), [DOCUMENT_MOVED_PATH]);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

#[test]
fn a_hand_commit_that_copies_a_document_to_a_move_s_destination_is_not_the_move_s_own() {
    let (world, retry) = interrupted_move();
    // Someone commits the document at the destination, as the move would
    // write it, and leaves it where it was.
    let copied = world.commit_in_context(
        DOCUMENT,
        DOCUMENT_MOVED_PATH,
        &world.primary_source_at(DOCUMENT_PATH),
    );
    assert_eq!(
        world.documents_at(copied),
        [DOCUMENT_PATH, DOCUMENT_MOVED_PATH]
    );

    let envelope = world.execute(REQUEST_1, retry);
    // The domain refuses the retry: the file at the destination is not
    // the bytes the move writes. The copy still holds the document where
    // the move takes it from, so it is not named as the move's commit.
    assert_eq!(envelope.code, ResultCode::ExternalChange);
    assert_eq!(envelope.effects.commit_oid, None);
    assert_eq!(
        envelope.effects.checkpoint,
        CheckpointEffect::Pending,
        "no commit is the request's"
    );
    assert_eq!(world.branch_tip(DOCUMENT), Some(copied));
}
