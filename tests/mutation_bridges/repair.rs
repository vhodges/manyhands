//! Repair and adoption: `repair_item`, and `document repair` and
//! `ticket repair` through the boundary.

use std::path::PathBuf;

use manyhands::{
    repository::{
        AuthoringKind, ExpectedPathObservation, FailurePoint, ItemContextKind, LocalCheckpoint,
        Mutation, MutationDataDto, OperationAbandonInput, OperationAction, OperationFamily,
        OperationId, OperationResumeInput, PreviewDetailsDto, PreviewEffect, RepairItemRequest,
        RepairPreviewDto, RepositoryError, RepositoryErrorKind, RequestState, SaveOutcome,
        TicketFilter, TicketRepairInput,
    },
    results::{CheckpointEffect, DiscoveryEffect, Envelope, Outcome, ResultCode, WriteEffect},
};

use super::support::{
    self, git_home,
    golden::ContractCase,
    items,
    mutation::{self, REQUEST_1, REQUEST_2, REQUEST_3, World, execute_with, recovery_actions},
    repair::*,
    schema,
};

const REPAIR_SCHEMA: &str = "repair_mutation.schema.json";
const PREVIEW_SCHEMA: &str = "repair_preview.schema.json";

fn kind_of(result: Result<SaveOutcome, RepositoryError>) -> RepositoryErrorKind {
    match result {
        Ok(_) => panic!("the repair was made"),
        Err(error) => {
            let text = format!("{error} {error:?}");
            for sentinel in SENTINELS {
                assert!(!text.contains(sentinel), "the error holds {sentinel}");
            }
            error.kind
        }
    }
}

fn request(shelf: &Shelf, kind: AuthoringKind, path: &str, source: &str) -> RepairItemRequest {
    RepairItemRequest {
        root: shelf.root.clone(),
        kind,
        path: PathBuf::from(path),
        source: source.to_owned(),
        expected: shelf.expected(path),
        operation_id: support::new_operation_id(),
    }
}

mod domain {
    use super::*;

    #[test]
    fn a_marker_only_document_is_repaired_in_the_context_of_the_id_its_source_carries() {
        git_home::isolated();
        let shelf = Shelf::new();
        let before = shelf.primary(MARKER_PATH);
        let head = shelf.primary_head();
        let source = document(ADOPTED);

        let outcome = shelf
            .service
            .repair_item(request(
                &shelf,
                AuthoringKind::Document,
                MARKER_PATH,
                &source,
            ))
            .unwrap();

        let SaveOutcome::Saved {
            context,
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
        } = outcome
        else {
            panic!("the repair was not checkpointed");
        };
        assert_eq!(context.item_id.to_string(), ADOPTED);
        assert_eq!(context.branch, World::document_branch(ADOPTED));
        assert_eq!(context.worktree, shelf.worktree(ADOPTED));
        // The context holds the source, byte for byte, in one commit.
        assert_eq!(
            shelf.context(ADOPTED, MARKER_PATH).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.branch_tip(ADOPTED), Some(commit_oid));
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(
            shelf.tip_subject(ADOPTED),
            format!("Checkpoint document {ADOPTED}")
        );
        // Primary is as it was.
        assert_eq!(shelf.primary(MARKER_PATH), before);
        assert_eq!(shelf.primary_head(), head);
        assert_eq!(shelf.pending_journal_rows(), 0);
    }

    #[test]
    fn a_marker_only_ticket_is_repaired_with_the_id_of_its_directory() {
        git_home::isolated();
        let shelf = Shelf::new();
        let path = items::ticket_path(TICKET_MARKER);
        let source = ticket(TICKET_MARKER, "");

        let outcome = shelf
            .service
            .repair_item(request(&shelf, AuthoringKind::Ticket, &path, &source))
            .unwrap();

        assert!(matches!(
            outcome,
            SaveOutcome::Saved {
                checkpoint: LocalCheckpoint::Checkpointed { .. },
                ..
            }
        ));
        assert_eq!(
            shelf.context(TICKET_MARKER, &path).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(
            shelf.tip_subject(TICKET_MARKER),
            format!("Checkpoint ticket {TICKET_MARKER}")
        );
    }

    #[test]
    fn a_refused_repair_writes_nothing_and_makes_no_context() {
        git_home::isolated();
        let shelf = Shelf::new();
        let doc = AuthoringKind::Document;
        let tic = AuthoringKind::Ticket;
        let marker_ticket = items::ticket_path(TICKET_MARKER);
        let closed = items::ticket_path(TICKET_CLOSED_BROKEN);
        let cases: Vec<(&str, AuthoringKind, &str, String, RepositoryErrorKind)> = vec![
            (
                "an ID another item holds",
                doc,
                MARKER_PATH,
                document(mutation::DOCUMENT),
                RepositoryErrorKind::AmbiguousIdentity,
            ),
            (
                "an ID a ticket holds",
                doc,
                MARKER_PATH,
                document(items::TICKET_A),
                RepositoryErrorKind::AmbiguousIdentity,
            ),
            (
                "a file that holds another valid ID",
                doc,
                UNTITLED_PATH,
                document(UNHELD),
                RepositoryErrorKind::NotRepairable,
            ),
            (
                "a file that is an item already",
                doc,
                mutation::DOCUMENT_PATH,
                document(mutation::DOCUMENT),
                RepositoryErrorKind::NotRepairable,
            ),
            (
                "a file of another kind than its path's",
                doc,
                MISPLACED_PATH,
                document(MISPLACED),
                RepositoryErrorKind::NotRepairable,
            ),
            (
                "a managed file outside every canonical path",
                doc,
                OUTSIDE_PATH,
                document(UNHELD),
                RepositoryErrorKind::NotRepairable,
            ),
            (
                "a source that is itself nonconforming",
                doc,
                MARKER_PATH,
                marker_only(),
                RepositoryErrorKind::InvalidItemSource,
            ),
            (
                "a source of the wrong kind for the path",
                doc,
                MARKER_PATH,
                ticket(UNHELD, ""),
                RepositoryErrorKind::InvalidItemSource,
            ),
            (
                "a ticket of another ID than its directory's",
                tic,
                &marker_ticket,
                ticket(UNHELD, ""),
                RepositoryErrorKind::NotRepairable,
            ),
            (
                "a ticket directory that is no ULID",
                tic,
                TICKET_UNNAMED_PATH,
                ticket(UNHELD, ""),
                RepositoryErrorKind::NotRepairable,
            ),
            (
                "a closed ticket's closure dropped",
                tic,
                &closed,
                ticket(TICKET_CLOSED_BROKEN, ""),
                RepositoryErrorKind::InvalidItemSource,
            ),
        ];
        for (what, kind, path, source, expected) in cases {
            let refused = shelf
                .service
                .repair_item(request(&shelf, kind, path, &source));
            assert_eq!(kind_of(refused), expected, "{what}");
            assert_eq!(shelf.contexts(), Vec::<String>::new(), "{what}: a context");
            assert_eq!(
                shelf.item_branches(),
                Vec::<String>::new(),
                "{what}: a branch"
            );
            assert_eq!(shelf.pending_journal_rows(), 0, "{what}: a pending row");
        }
    }

    #[test]
    fn an_id_held_only_in_its_own_context_is_ambiguous_and_the_context_is_left() {
        git_home::isolated();
        let shelf = Shelf::new();
        // A document that exists in its editing context and not on
        // primary: the index is not asked here, and primary does not
        // hold its ID.
        let created = shelf.execute(
            REQUEST_1,
            shelf.document_create(mutation::DOCUMENT_NEW, mutation::DOCUMENT_NEW_PATH),
        );
        assert_eq!(created.code, ResultCode::Ok);
        let tip = shelf.branch_tip(mutation::DOCUMENT_NEW);
        let held = shelf.context(mutation::DOCUMENT_NEW, MARKER_PATH);

        let refused = shelf.service.repair_item(request(
            &shelf,
            AuthoringKind::Document,
            MARKER_PATH,
            &document(mutation::DOCUMENT_NEW),
        ));

        assert_eq!(kind_of(refused), RepositoryErrorKind::AmbiguousIdentity);
        // No second file of the ID was written beside the first.
        assert_eq!(shelf.context(mutation::DOCUMENT_NEW, MARKER_PATH), held);
        assert_eq!(held, Some(shelf.primary(MARKER_PATH)));
        assert_eq!(shelf.branch_tip(mutation::DOCUMENT_NEW), tip);
        assert_eq!(shelf.pending_journal_rows(), 0);
    }

    #[test]
    fn a_file_that_is_not_what_the_caller_saw_is_an_external_change() {
        git_home::isolated();
        let shelf = Shelf::new();
        let request = RepairItemRequest {
            expected: ExpectedPathObservation::from_bytes(b"something else"),
            ..request(
                &shelf,
                AuthoringKind::Document,
                MARKER_PATH,
                &document(ADOPTED),
            )
        };

        assert_eq!(
            kind_of(shelf.service.repair_item(request)),
            RepositoryErrorKind::ExternalChange
        );
        assert!(shelf.contexts().is_empty());
        assert!(shelf.item_branches().is_empty());
        assert_eq!(shelf.pending_journal_rows(), 0);
    }

    #[test]
    fn a_repair_stopped_after_its_write_is_completed_by_a_repeat_of_its_operation() {
        git_home::isolated();
        let shelf = Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let source = document(ADOPTED);
        let request = request(&shelf, AuthoringKind::Document, MARKER_PATH, &source);

        let stopped = shelf.service.repair_item(request.clone());

        assert_eq!(kind_of(stopped), RepositoryErrorKind::Sqlite);
        // Written and not committed, with its row pending.
        assert_eq!(
            shelf.context(ADOPTED, MARKER_PATH).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.branch_commits(ADOPTED), 0);
        assert_eq!(shelf.pending_journal_rows(), 1);

        let outcome = shelf.service.repair_item(request).unwrap();

        assert!(matches!(
            outcome,
            SaveOutcome::Saved {
                checkpoint: LocalCheckpoint::Checkpointed { .. },
                ..
            }
        ));
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(shelf.contexts(), [ADOPTED]);
        assert_eq!(shelf.pending_journal_rows(), 0);
    }

    #[test]
    fn a_save_is_not_given_the_intent_of_a_repair() {
        git_home::isolated();
        let shelf = Shelf::new();
        let rows = shelf.journal_rows();

        let refused = shelf
            .service
            .save_document(manyhands::repository::SaveDocumentRequest {
                target: manyhands::repository::AuthoringTarget {
                    root: shelf.root.clone(),
                    kind: AuthoringKind::Document,
                    item_id: items::item_id(ADOPTED),
                    intent: manyhands::repository::ContextIntent::Repair,
                    operation_id: support::new_operation_id(),
                },
                source_path: None,
                destination_path: PathBuf::from(MARKER_PATH),
                draft: World::document_draft(),
                expected_source: None,
                expected_destination: shelf.expected(MARKER_PATH),
            });

        assert_eq!(kind_of(refused), RepositoryErrorKind::OperationMismatch);
        assert_eq!(shelf.journal_rows(), rows);
        assert!(shelf.contexts().is_empty());
    }
}

/// Checks an envelope of a repair, or of its preview, against its
/// fixture. What differs from run to run is replaced.
fn assert_golden<T: serde::Serialize>(
    name: &str,
    shelf: &Shelf,
    data_schema: &str,
    envelope: &Envelope<T>,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let text = |pointer: &str| value.pointer(pointer).and_then(|found| found.as_str());
    let mut placeholders = vec![(shelf.root.to_str().unwrap(), "<root>")];
    for (pointer, placeholder) in [
        ("/data/confirmation_id", "<confirmation>"),
        ("/data/expires_at", "<expires>"),
        ("/effects/commit_oid", "<commit>"),
        ("/operation_id", "<operation>"),
    ] {
        if let Some(found) = text(pointer) {
            placeholders.push((found, placeholder));
        }
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: (!value["data"].is_null()).then_some(data_schema),
            placeholders: &placeholders,
            sentinels: &SENTINELS,
        },
        envelope,
    );
}

/// Asserts that a refused repair left nothing: no file changed on
/// primary, no context, no branch, no journal row and no record.
fn assert_nothing_made(shelf: &Shelf, journal_rows: i64, what: &str) {
    assert_eq!(shelf.contexts(), Vec::<String>::new(), "{what}: a context");
    assert_eq!(
        shelf.item_branches(),
        Vec::<String>::new(),
        "{what}: a branch"
    );
    assert_eq!(shelf.journal_rows(), journal_rows, "{what}: a journal row");
    assert_eq!(shelf.request_rows(), 0, "{what}: a request record");
    assert_eq!(shelf.confirmation_rows(), 0, "{what}: a confirmation");
}

/// Asserts that `prepare` refuses `mutation` as `code`, with nothing
/// made.
fn assert_not_previewed(shelf: &Shelf, mutation: &Mutation, code: ResultCode, what: &str) {
    let rows = shelf.journal_rows();
    let before = support::repository_and_worktree_snapshot(&shelf.fixture);
    let refused = shelf.prepare(mutation);
    assert_eq!(refused.code, code, "{what}");
    assert!(
        matches!(refused.outcome, Outcome::Error | Outcome::Blocked),
        "{what}: {:?}",
        refused.outcome
    );
    assert!(refused.data.is_none(), "{what}: a preview");
    assert_nothing_made(shelf, rows, what);
    assert!(
        support::repository_and_worktree_snapshot(&shelf.fixture) == before,
        "{what}: the repository"
    );
}

mod boundary {
    use super::*;

    #[test]
    fn a_marker_only_document_is_adopted_with_the_id_its_source_carries() {
        git_home::isolated();
        let shelf = Shelf::new();
        let before = shelf.primary(MARKER_PATH);
        let head = shelf.primary_head();
        let rows = shelf.journal_rows();
        let source = document(ADOPTED);
        let repair = || shelf.document_repair(MARKER_PATH, &source);

        let previewed = shelf.prepare(&repair());

        assert_eq!(previewed.code, ResultCode::Ok);
        assert_eq!(previewed.outcome, Outcome::Success);
        let preview = previewed.data.clone().unwrap();
        assert_eq!(preview.command, "document repair");
        // The preview states the adoption and the ID.
        assert_eq!(
            preview
                .effects
                .iter()
                .map(|entry| entry.effect)
                .collect::<Vec<_>>(),
            [
                PreviewEffect::ContextCreated,
                PreviewEffect::IdAdopted,
                PreviewEffect::SourceWritten,
                PreviewEffect::CheckpointCommitted,
            ]
        );
        assert_eq!(
            preview.details,
            PreviewDetailsDto::Repair(RepairPreviewDto {
                root: shelf.root.to_str().unwrap().to_owned(),
                path: MARKER_PATH.to_owned(),
                id: ADOPTED.to_owned(),
                adoption: true,
                primary_changed: false,
            })
        );
        assert_eq!(preview.scope.item_id.as_deref(), Some(ADOPTED));
        // A preview makes nothing but its confirmation.
        assert!(shelf.contexts().is_empty());
        assert!(shelf.item_branches().is_empty());
        assert_eq!(shelf.journal_rows(), rows);
        assert_eq!(shelf.request_rows(), 0);
        assert_eq!(shelf.confirmation_rows(), 1);
        assert_golden(
            "document_repair_preview",
            &shelf,
            PREVIEW_SCHEMA,
            &previewed,
        );

        let done = shelf.confirmed(REQUEST_1, &preview.confirmation_id, repair());

        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(done.outcome, Outcome::Success);
        assert_eq!(done.effects.write, WriteEffect::Written);
        assert_eq!(done.effects.checkpoint, CheckpointEffect::Committed);
        assert_eq!(done.effects.discovery, DiscoveryEffect::Current);
        let data = repair_data(&done);
        assert_eq!(data.id, ADOPTED);
        assert_eq!(data.path, MARKER_PATH);
        assert_eq!(data.adopted, Some(true));
        assert_eq!(done.scope.item_id.as_deref(), Some(ADOPTED));
        assert_eq!(
            done.scope.branch.as_deref(),
            Some(World::document_branch(ADOPTED).as_str())
        );
        // The item's context holds the conforming file, in one
        // checkpoint commit.
        assert_eq!(
            shelf.context(ADOPTED, MARKER_PATH).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.contexts(), [ADOPTED]);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(
            done.effects.commit_oid,
            shelf.branch_tip(ADOPTED).map(|tip| tip.to_string())
        );
        assert_eq!(
            shelf.tip_subject(ADOPTED),
            format!("Checkpoint document {ADOPTED}")
        );
        // Primary is unchanged.
        assert_eq!(shelf.primary(MARKER_PATH), before);
        assert_eq!(shelf.primary_head(), head);
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Finished
        );
        assert_eq!(shelf.pending_journal_rows(), 0);
        assert_golden("document_repair", &shelf, REPAIR_SCHEMA, &done);

        // Asked again, the repair is made: no preview is given of it.
        let again = shelf.prepare(&repair());
        assert_eq!(again.code, ResultCode::AlreadyApplied);
        assert_eq!(again.outcome, Outcome::Noop);
        assert!(again.data.is_none());
        assert_golden(
            "document_repair_preview_noop",
            &shelf,
            PREVIEW_SCHEMA,
            &again,
        );
    }

    #[test]
    fn a_repair_stopped_after_its_write_is_completed_by_its_retry_with_the_same_id() {
        git_home::isolated();
        let mut shelf =
            Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, preview) = shelf.preview(&repair(&shelf));
        assert_eq!(preview.scope.item_id.as_deref(), Some(ADOPTED));

        let stopped = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));

        // The source is written and not committed.
        assert_eq!(stopped.code, ResultCode::InternalError);
        assert_eq!(stopped.outcome, Outcome::Partial);
        assert_eq!(stopped.effects.write, WriteEffect::Written);
        assert_eq!(stopped.effects.checkpoint, CheckpointEffect::Pending);
        assert_eq!(repair_data(&stopped).id, ADOPTED);
        assert_eq!(
            mutation::recovery_before_own_abandon(&stopped),
            ["request.retry"]
        );
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        assert_eq!(shelf.branch_commits(ADOPTED), 0);
        assert_eq!(shelf.pending_journal_rows(), 1);
        assert_golden("document_repair_partial", &shelf, REPAIR_SCHEMA, &stopped);

        // The same request again, as a new process sends it, with no
        // confirmation: its record holds the one it was accepted with.
        shelf.reopen();
        let done = execute_with(&shelf.service, REQUEST_1, repair(&shelf));

        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(done.outcome, Outcome::Success);
        assert_eq!(done.operation_id, stopped.operation_id);
        let data = repair_data(&done);
        assert_eq!(data.id, ADOPTED);
        assert_eq!(data.adopted, Some(true));
        assert_eq!(
            done.effects.commit_oid,
            shelf.branch_tip(ADOPTED).map(|tip| tip.to_string())
        );
        // One context, of the one ID, with one commit.
        assert_eq!(shelf.contexts(), [ADOPTED]);
        assert_eq!(shelf.item_branches(), [World::document_branch(ADOPTED)]);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(shelf.pending_journal_rows(), 0);
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Finished
        );
    }

    #[test]
    fn a_repair_lost_with_the_index_is_made_again_under_the_same_id_in_the_same_context() {
        git_home::isolated();
        let mut shelf =
            Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, first) = shelf.preview(&repair(&shelf));
        let stopped = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(stopped.outcome, Outcome::Partial);
        assert_eq!(shelf.contexts(), [ADOPTED]);

        // The database is deleted, and the repository rebuilt.
        shelf.lose_cache();
        let unregistered = shelf.prepare(&repair(&shelf));
        assert_eq!(unregistered.code, ResultCode::RepositoryNotRegistered);
        shelf.rebuild();
        assert!(shelf.record(REQUEST_1).is_none());
        assert_eq!(
            shelf.journal_rows(),
            shelf.count(
                "SELECT COUNT(*) FROM operation_records WHERE action IN ('rebuild', 'refresh')"
            )
        );

        // A new preview and a new request, with the same source.
        let (confirmation, second) = shelf.preview(&repair(&shelf));
        assert_ne!(second.confirmation_id, first.confirmation_id);
        assert_eq!(second.details, first.details);
        assert_eq!(second.scope.item_id.as_deref(), Some(ADOPTED));
        // The context is there already: none is made.
        assert_eq!(
            second
                .effects
                .iter()
                .map(|entry| entry.effect)
                .collect::<Vec<_>>(),
            [
                PreviewEffect::IdAdopted,
                PreviewEffect::SourceWritten,
                PreviewEffect::CheckpointCommitted,
            ]
        );
        let done = shelf.confirmed(REQUEST_2, &confirmation, repair(&shelf));

        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(done.outcome, Outcome::Success);
        assert_eq!(repair_data(&done).id, ADOPTED);
        assert_eq!(done.scope.item_id.as_deref(), Some(ADOPTED));
        assert_eq!(shelf.contexts(), [ADOPTED]);
        assert_eq!(shelf.item_branches(), [World::document_branch(ADOPTED)]);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(
            shelf.context(ADOPTED, MARKER_PATH).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.pending_journal_rows(), 0);
    }

    #[test]
    fn a_stale_token_is_an_external_change_at_prepare() {
        git_home::isolated();
        let shelf = Shelf::new();
        let stale = shelf.document_repair_with(MARKER_PATH, "v1:stale", &document(ADOPTED));

        assert_not_previewed(&shelf, &stale, ResultCode::ExternalChange, "a stale token");

        // And the token of the file as it was, once the file has changed.
        let token = shelf.path_token(MARKER_PATH);
        shelf.commit_on_primary(MARKER_PATH, "---\nmanyhands_managed: true\n---\nEdited.\n");
        let late =
            shelf.prepare(&shelf.document_repair_with(MARKER_PATH, &token, &document(ADOPTED)));
        assert_eq!(late.code, ResultCode::ExternalChange);
        assert!(late.data.is_none());
        assert_eq!(shelf.confirmation_rows(), 0);
    }

    #[test]
    fn the_same_request_with_another_source_is_a_request_mismatch() {
        git_home::isolated();
        let shelf = Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let token = shelf.path_token(MARKER_PATH);
        let repair =
            |source: &str| shelf.document_repair_with(MARKER_PATH, &token, &document(source));
        let (confirmation, _) = shelf.preview(&repair(ADOPTED));
        let stopped = shelf.confirmed(REQUEST_1, &confirmation, repair(ADOPTED));
        assert_eq!(stopped.outcome, Outcome::Partial);
        let record = shelf.record(REQUEST_1);

        // Another source, with another ID, under the accepted request.
        let mismatch = shelf.confirmed(REQUEST_1, &confirmation, repair(UNHELD));

        assert_eq!(mismatch.code, ResultCode::RequestMismatch);
        assert_eq!(shelf.record(REQUEST_1), record);
        assert_eq!(shelf.contexts(), [ADOPTED]);
        assert_eq!(shelf.branch_commits(ADOPTED), 0);
    }

    #[test]
    fn a_repaired_document_is_listed_twice_and_shown_from_its_context() {
        git_home::isolated();
        let shelf = Shelf::new();
        let source = document(ADOPTED);
        let done = shelf.repair(REQUEST_1, || shelf.document_repair(MARKER_PATH, &source));
        assert_eq!(done.code, ResultCode::Ok);

        let listed = shelf.service.list_documents(&shelf.resolved()).unwrap();
        let here: Vec<_> = listed
            .items
            .iter()
            .filter(|item| item.path == MARKER_PATH)
            .collect();
        // The nonconforming entry from primary, by path, and the item
        // from its context.
        assert_eq!(here.len(), 2);
        assert_eq!(here[0].id, None);
        assert_eq!(here[0].context.kind, ItemContextKind::Primary);
        assert!(!here[0].problems.is_empty());
        assert_eq!(here[1].id.as_deref(), Some(ADOPTED));
        assert_eq!(here[1].context.kind, ItemContextKind::Active);
        assert_eq!(
            here[1].context.branch.as_deref(),
            Some(World::document_branch(ADOPTED).as_str())
        );
        assert_ne!(here[1].context.worktree, here[0].context.worktree);

        // By its new ID the item is the context's copy.
        let shown = shelf
            .service
            .show_item(&shelf.resolved(), &items::item_id(ADOPTED))
            .unwrap();
        assert_eq!(shown.source.as_deref(), Some(source.as_str()));
        assert_eq!(shown.path, MARKER_PATH);
        assert_eq!(shown.context.kind, ItemContextKind::Active);
        assert!(shown.problems.is_empty());
    }

    #[test]
    fn a_repaired_ticket_is_listed_once() {
        git_home::isolated();
        let shelf = Shelf::new();
        let path = items::ticket_path(TICKET_MARKER);
        let source = ticket(TICKET_MARKER, "");
        let before = shelf
            .service
            .list_tickets(&shelf.resolved(), &TicketFilter::default())
            .unwrap();
        assert_eq!(
            before
                .items
                .iter()
                .filter(|item| item.path == path)
                .map(|item| item.id.clone())
                .collect::<Vec<_>>(),
            [None]
        );

        let done = shelf.repair(REQUEST_1, || shelf.ticket_repair(&path, &source));
        assert_eq!(done.code, ResultCode::Ok);

        let listed = shelf
            .service
            .list_tickets(&shelf.resolved(), &TicketFilter::default())
            .unwrap();
        let here: Vec<_> = listed
            .items
            .iter()
            .filter(|item| item.path == path)
            .collect();
        assert_eq!(here.len(), 1);
        assert_eq!(here[0].id.as_deref(), Some(TICKET_MARKER));
        assert_eq!(here[0].context.kind, ItemContextKind::Active);
    }

    #[test]
    fn an_ambiguous_identity_is_refused_and_nothing_is_made() {
        git_home::isolated();
        let shelf = Shelf::new();
        // A source whose ID another item holds: a document, a ticket.
        for held in [mutation::DOCUMENT, items::TICKET_A, DUPLICATE] {
            assert_not_previewed(
                &shelf,
                &shelf.document_repair(MARKER_PATH, &document(held)),
                ResultCode::IdentityAmbiguous,
                held,
            );
        }
        // A path under a duplicate-ID problem, whatever ID the source
        // carries.
        for (path, id) in [
            (DUPLICATE_PATHS[0], DUPLICATE),
            (DUPLICATE_PATHS[1], UNHELD),
        ] {
            assert_not_previewed(
                &shelf,
                &shelf.document_repair(path, &document(id)),
                ResultCode::IdentityAmbiguous,
                path,
            );
        }
        let refused = shelf.prepare(&shelf.document_repair(MARKER_PATH, &document(DUPLICATE)));
        assert!(recovery_actions_of(&refused).is_empty());
        assert_golden(
            "document_repair_identity_ambiguous",
            &shelf,
            PREVIEW_SCHEMA,
            &refused,
        );
    }

    fn recovery_actions_of<T>(envelope: &Envelope<T>) -> Vec<&'static str> {
        envelope
            .recovery
            .iter()
            .map(|action| action.action.as_str())
            .collect()
    }

    #[test]
    fn an_identity_taken_after_the_preview_is_an_external_change() {
        git_home::isolated();
        let shelf = Shelf::new();
        let source = document(mutation::DOCUMENT_NEW);
        let token = shelf.path_token(MARKER_PATH);
        let repair = || shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, _) = shelf.preview(&repair());
        let rows = shelf.journal_rows();

        // Another item is created with the ID the preview was of.
        let created = shelf.execute(
            REQUEST_2,
            shelf.document_create(mutation::DOCUMENT_NEW, mutation::DOCUMENT_NEW_PATH),
        );
        assert_eq!(created.code, ResultCode::Ok);
        let made = shelf.journal_rows() - rows;

        let refused = shelf.confirmed(REQUEST_1, &confirmation, repair());

        assert_eq!(refused.code, ResultCode::ExternalChange);
        assert_eq!(recovery_actions(&refused), ["request.prepare"]);
        assert!(shelf.record(REQUEST_1).is_none());
        assert_eq!(shelf.journal_rows(), rows + made);
        // The context is the created document's, and holds no repair.
        assert_eq!(shelf.contexts(), [mutation::DOCUMENT_NEW]);
        assert_eq!(
            shelf.context(mutation::DOCUMENT_NEW, MARKER_PATH),
            Some(shelf.primary(MARKER_PATH))
        );
        // A new preview says what stands in the way.
        let again = shelf.prepare(&repair());
        assert_eq!(again.code, ResultCode::IdentityAmbiguous);
    }

    #[test]
    fn a_file_with_a_valid_id_keeps_it() {
        git_home::isolated();
        let shelf = Shelf::new();

        // A source with another ID.
        assert_not_previewed(
            &shelf,
            &shelf.document_repair(UNTITLED_PATH, &document(UNHELD)),
            ResultCode::NotRepairable,
            "another ID",
        );

        // The source that keeps it is no adoption.
        let source = document(UNTITLED);
        let repair = || shelf.document_repair(UNTITLED_PATH, &source);
        let (confirmation, preview) = shelf.preview(&repair());
        assert_eq!(
            preview.details,
            PreviewDetailsDto::Repair(RepairPreviewDto {
                root: shelf.root.to_str().unwrap().to_owned(),
                path: UNTITLED_PATH.to_owned(),
                id: UNTITLED.to_owned(),
                adoption: false,
                primary_changed: false,
            })
        );
        assert!(
            preview
                .effects
                .iter()
                .all(|entry| entry.effect != PreviewEffect::IdAdopted)
        );
        let done = shelf.confirmed(REQUEST_1, &confirmation, repair());
        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(repair_data(&done).adopted, Some(false));
        assert_eq!(
            shelf.context(UNTITLED, UNTITLED_PATH).as_deref(),
            Some(source.as_bytes())
        );
    }

    #[test]
    fn a_source_that_is_nonconforming_or_of_the_wrong_kind_is_invalid_input() {
        git_home::isolated();
        let shelf = Shelf::new();
        let ticket_path = items::ticket_path(TICKET_MARKER);
        let cases = [
            (
                "a source that is itself nonconforming",
                shelf.document_repair(MARKER_PATH, &marker_only()),
            ),
            (
                "a source with no front matter",
                shelf.document_repair(MARKER_PATH, "Only text.\n"),
            ),
            (
                "a ticket's source for a document's path",
                shelf.document_repair(MARKER_PATH, &ticket(UNHELD, "")),
            ),
            (
                "a document's source for a ticket's path",
                shelf.ticket_repair(&ticket_path, &document(TICKET_MARKER)),
            ),
        ];
        for (what, mutation) in &cases {
            assert_not_previewed(&shelf, mutation, ResultCode::InvalidInput, what);
        }
    }

    #[test]
    fn an_uncommitted_nonconforming_file_is_worktree_not_clean() {
        git_home::isolated();
        let shelf = Shelf::new();
        let rows = shelf.journal_rows();

        // A file that was never committed.
        items::write(&shelf.root, "docs/draft.md", &marker_only());
        let untracked = shelf.prepare(&shelf.document_repair("docs/draft.md", &document(ADOPTED)));
        assert_eq!(untracked.code, ResultCode::WorktreeNotClean);
        assert!(untracked.data.is_none());

        // A committed file that was edited since.
        items::write(
            &shelf.root,
            MARKER_PATH,
            "---\nmanyhands_managed: true\n---\nEdited and not committed.\n",
        );
        let edited = shelf.prepare(&shelf.document_repair(MARKER_PATH, &document(ADOPTED)));
        assert_eq!(edited.code, ResultCode::WorktreeNotClean);
        assert!(edited.data.is_none());
        assert_nothing_made(&shelf, rows, "an uncommitted file");
    }

    #[test]
    fn a_marker_only_ticket_is_repaired_with_the_id_of_its_directory_and_no_other() {
        git_home::isolated();
        let shelf = Shelf::new();
        let path = items::ticket_path(TICKET_MARKER);
        let unlisted = |path: &str, source: &str| {
            Mutation::TicketRepair(TicketRepairInput {
                root: shelf.root.clone(),
                path: PathBuf::from(path),
                observation: "v1:unread".to_owned(),
                source: source.to_owned(),
            })
        };

        // Another ID than the directory's.
        assert_not_previewed(
            &shelf,
            &shelf.ticket_repair(&path, &ticket(UNHELD, "")),
            ResultCode::NotRepairable,
            "another ID",
        );
        // A directory that is no ULID, whatever the source.
        for source in [ticket(UNHELD, ""), ticket(TICKET_MARKER, "")] {
            assert_not_previewed(
                &shelf,
                &unlisted(TICKET_UNNAMED_PATH, &source),
                ResultCode::NotRepairable,
                "a directory that is no ULID",
            );
        }
        let refused = shelf.prepare(&unlisted(TICKET_UNNAMED_PATH, &ticket(UNHELD, "")));
        assert_golden(
            "ticket_repair_not_repairable",
            &shelf,
            PREVIEW_SCHEMA,
            &refused,
        );

        // The directory's ID.
        let source = ticket(TICKET_MARKER, "");
        let repair = || shelf.ticket_repair(&path, &source);
        let previewed = shelf.prepare(&repair());
        let preview = previewed.data.clone().unwrap();
        assert_eq!(preview.command, "ticket repair");
        assert_eq!(
            preview.details,
            PreviewDetailsDto::Repair(RepairPreviewDto {
                root: shelf.root.to_str().unwrap().to_owned(),
                path: path.clone(),
                id: TICKET_MARKER.to_owned(),
                adoption: true,
                primary_changed: false,
            })
        );
        assert_golden("ticket_repair_preview", &shelf, PREVIEW_SCHEMA, &previewed);
        let done = shelf.confirmed(REQUEST_1, &preview.confirmation_id, repair());

        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(done.outcome, Outcome::Success);
        assert_eq!(repair_data(&done).id, TICKET_MARKER);
        assert_eq!(
            done.scope.branch.as_deref(),
            Some(World::branch(TICKET_MARKER).as_str())
        );
        assert_eq!(
            shelf.context(TICKET_MARKER, &path).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.branch_commits(TICKET_MARKER), 1);
        assert_eq!(
            shelf.tip_subject(TICKET_MARKER),
            format!("Checkpoint ticket {TICKET_MARKER}")
        );
        assert_golden("ticket_repair", &shelf, REPAIR_SCHEMA, &done);
    }

    #[test]
    fn a_closed_ticket_is_repaired_keeping_its_closure_and_no_repair_closes_or_reopens() {
        git_home::isolated();
        let shelf = Shelf::new();
        let closed = items::ticket_path(TICKET_CLOSED_BROKEN);
        let open = items::ticket_path(TICKET_OPEN_BROKEN);
        let unreadable = items::ticket_path(TICKET_UNREADABLE);
        let other_closure =
            "closed_at: 2026-09-30T12:34:57Z\nclosed_by: Ada Lovelace <ada@example.invalid>\n";

        for (what, path, source) in [
            (
                "a source that drops the closure",
                &closed,
                ticket(TICKET_CLOSED_BROKEN, ""),
            ),
            (
                "a source that changes the closure",
                &closed,
                ticket(TICKET_CLOSED_BROKEN, other_closure),
            ),
            (
                "a source that closes an open ticket",
                &open,
                ticket(TICKET_OPEN_BROKEN, items::CLOSURE),
            ),
            (
                "closure fields for front matter that cannot be read",
                &unreadable,
                ticket(TICKET_UNREADABLE, items::CLOSURE),
            ),
        ] {
            assert_not_previewed(
                &shelf,
                &shelf.ticket_repair(path, &source),
                ResultCode::InvalidInput,
                what,
            );
        }

        // The closed ticket, with its closure as it is.
        let source = ticket(TICKET_CLOSED_BROKEN, items::CLOSURE);
        let done = shelf.repair(REQUEST_1, || shelf.ticket_repair(&closed, &source));
        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(repair_data(&done).adopted, Some(false));
        assert_eq!(
            shelf.context(TICKET_CLOSED_BROKEN, &closed).as_deref(),
            Some(source.as_bytes())
        );
        let shown = shelf
            .service
            .show_item(&shelf.resolved(), &items::item_id(TICKET_CLOSED_BROKEN))
            .unwrap();
        assert!(shown.closure.is_some());

        // The open one, left open; and the unreadable one, adopted.
        let source = ticket(TICKET_OPEN_BROKEN, "");
        let done = shelf.repair(REQUEST_2, || shelf.ticket_repair(&open, &source));
        assert_eq!(done.code, ResultCode::Ok);
        let source = ticket(TICKET_UNREADABLE, "");
        let done = shelf.repair(REQUEST_3, || shelf.ticket_repair(&unreadable, &source));
        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(repair_data(&done).adopted, Some(true));
    }

    #[test]
    fn a_managed_file_outside_its_canonical_path_is_not_repairable() {
        git_home::isolated();
        let shelf = Shelf::new();
        let outside = |path: &str| shelf.document_repair_with(path, "v1:unread", &document(UNHELD));

        // Outside every canonical path, and at another kind's.
        for path in [
            OUTSIDE_PATH,
            "docs/notes.txt",
            ".manyhands/config.toml",
            &items::ticket_path(TICKET_MARKER),
        ] {
            assert_not_previewed(&shelf, &outside(path), ResultCode::NotRepairable, path);
        }
        // A path that is not written as one names nothing.
        for path in [
            "/docs/a.md",
            "docs/../a.md",
            ".manyhands/worktrees/x/docs/a.md",
        ] {
            assert_not_previewed(&shelf, &outside(path), ResultCode::InvalidPath, path);
        }
        // A file whose kind is another's than its path's.
        assert_not_previewed(
            &shelf,
            &shelf.document_repair(MISPLACED_PATH, &document(MISPLACED)),
            ResultCode::NotRepairable,
            "a ticket under docs",
        );
        // A file that is an item already.
        assert_not_previewed(
            &shelf,
            &shelf.document_repair(mutation::DOCUMENT_PATH, &document(mutation::DOCUMENT)),
            ResultCode::NotRepairable,
            "a conforming document",
        );
        // Nothing at the path.
        assert_not_previewed(
            &shelf,
            &outside("docs/absent.md"),
            ResultCode::PathNotFound,
            "an absent file",
        );
    }

    /// Unix only: a symbolic link is made.
    #[cfg(unix)]
    #[test]
    fn a_file_reached_through_a_symbolic_link_is_not_repaired() {
        git_home::isolated();
        let shelf = Shelf::new();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("outside.md");
        std::fs::write(&target, marker_only()).unwrap();
        std::os::unix::fs::symlink(&target, shelf.root.join("docs/link.md")).unwrap();
        std::os::unix::fs::symlink(outside.path(), shelf.root.join("docs/linked")).unwrap();
        let rows = shelf.journal_rows();

        for path in ["docs/link.md", "docs/linked/outside.md"] {
            let refused =
                shelf.prepare(&shelf.document_repair_with(path, "v1:unread", &document(ADOPTED)));
            assert_eq!(refused.code, ResultCode::InvalidPath, "{path}");
            assert!(refused.data.is_none());
        }
        assert_nothing_made(&shelf, rows, "a link");
        for path in ["docs/link.md", "docs/linked/outside.md"] {
            // The domain, asked directly, follows none either.
            let direct = shelf.service.repair_item(RepairItemRequest {
                root: shelf.root.clone(),
                kind: AuthoringKind::Document,
                path: PathBuf::from(path),
                source: document(ADOPTED),
                expected: ExpectedPathObservation::from_bytes(marker_only().as_bytes()),
                operation_id: support::new_operation_id(),
            });
            assert_eq!(
                kind_of(direct),
                RepositoryErrorKind::NotRepairable,
                "{path}"
            );
        }
        assert!(shelf.contexts().is_empty());
        assert!(shelf.item_branches().is_empty());
        assert_eq!(shelf.pending_journal_rows(), 0);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), marker_only());
    }

    #[test]
    fn a_repair_whose_result_was_lost_is_answered_with_its_commit_and_not_made_twice() {
        git_home::isolated();
        let mut shelf = Shelf::failing_once_at(FailurePoint::BeforeRequestSettlement);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, _) = shelf.preview(&repair(&shelf));

        // The repair is made, and the call dies before it says so.
        let lost = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(lost.code, ResultCode::InternalError);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        let tip = shelf.branch_tip(ADOPTED);
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        // Someone has edited primary's file since: the repair is done,
        // and is not held against what primary holds now.
        shelf.commit_on_primary(MARKER_PATH, "---\nmanyhands_managed: true\n---\nEdited.\n");
        shelf.reopen();

        let done = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));

        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(done.outcome, Outcome::Success);
        assert_eq!(done.effects.commit_oid, tip.map(|tip| tip.to_string()));
        assert_eq!(repair_data(&done).id, ADOPTED);
        assert_eq!(repair_data(&done).adopted, Some(true));
        assert_eq!(shelf.branch_tip(ADOPTED), tip);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Finished
        );
    }

    #[test]
    fn a_repair_stopped_before_its_commit_is_completed_whatever_primary_holds_by_then() {
        git_home::isolated();
        let mut shelf = Shelf::failing_once_at(FailurePoint::BeforeCheckpointCommit);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, _) = shelf.preview(&repair(&shelf));

        // The write is made and recorded, and the commit is not.
        let stopped = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(stopped.outcome, Outcome::Partial);
        assert_eq!(stopped.effects.write, WriteEffect::Written);
        assert_eq!(shelf.branch_commits(ADOPTED), 0);
        assert_eq!(
            shelf.count(
                "SELECT COUNT(*) FROM operation_records \
                  WHERE action = 'repair_item' \
                    AND completed_step = 'authoring_destination_observed'"
            ),
            1
        );
        // Primary's file is edited since. The source is in the item's
        // context, which no longer depends on what primary holds: the
        // request's own work is recorded, and nothing is compared.
        shelf.commit_on_primary(MARKER_PATH, "---\nmanyhands_managed: true\n---\nEdited.\n");
        shelf.reopen();

        let done = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));

        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(done.operation_id, stopped.operation_id);
        let data = repair_data(&done);
        assert_eq!(data.id, ADOPTED);
        assert_eq!(data.adopted, Some(true));
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(
            shelf.context(ADOPTED, MARKER_PATH).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.pending_journal_rows(), 0);
    }

    #[test]
    fn a_repair_that_died_before_its_domain_call_compares_again_when_it_is_entered_again() {
        git_home::isolated();
        let mut shelf = Shelf::failing_once_at(FailurePoint::BeforeRequestDomainCall);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, _) = shelf.preview(&repair(&shelf));
        let died = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(died.code, ResultCode::InternalError);
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        assert!(shelf.contexts().is_empty());

        // The file is changed on primary before the request is sent
        // again: what was consented to is no longer there.
        let edited = "---\nmanyhands_managed: true\n---\nEdited.\n";
        shelf.commit_on_primary(MARKER_PATH, edited);
        shelf.reopen();
        let refused = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));

        assert_eq!(refused.code, ResultCode::ExternalChange);
        assert_eq!(recovery_actions(&refused), ["request.prepare"]);
        // Nothing had been done, so nothing is kept.
        assert!(shelf.record(REQUEST_1).is_none());
        assert!(shelf.contexts().is_empty());
        assert!(shelf.item_branches().is_empty());
        assert_eq!(shelf.pending_journal_rows(), 0);

        // With the file as it was previewed, the request runs: the
        // confirmation was released with the record.
        shelf.commit_on_primary(MARKER_PATH, &marker_only());
        let done = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(done.code, ResultCode::Ok);
        assert_eq!(shelf.contexts(), [ADOPTED]);
    }

    #[test]
    fn a_repair_whose_refusal_was_lost_is_released_and_says_what_is_true_now() {
        git_home::isolated();
        let mut shelf = Shelf::failing_once_at(FailurePoint::BeforeRequestDomainCall);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        let (confirmation, _) = shelf.preview(&repair(&shelf));
        let died = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(died.code, ResultCode::InternalError);
        // The row of an operation that ended before its checkpoint,
        // having written nothing: one that stopped for want of an
        // identity leaves this, and its answer was lost.
        let operation = shelf.record(REQUEST_1).unwrap().operations[0]
            .operation_id
            .to_string();
        support::operations::insert_local(
            shelf.data.path(),
            Some(&operation),
            "repair_item",
            "completed",
            Some("worktree_observed"),
        );
        shelf.reopen();

        // Nothing has changed and nothing is in the way: the request
        // did not run, and can be sent again.
        let released = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));

        assert_eq!(released.code, ResultCode::Busy);
        assert_eq!(recovery_actions(&released), ["request.retry"]);
        assert!(shelf.record(REQUEST_1).is_none());
        assert!(shelf.contexts().is_empty());

        // Sent again it is a new request, under a new operation, and its
        // confirmation is its own to use again.
        let done = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(done.code, ResultCode::Ok);
        assert_ne!(done.operation_id.as_deref(), Some(operation.as_str()));
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
    }

    /// A repair stopped after its write: its request and its operation.
    fn interrupted(shelf: &Shelf, source: &str) -> (Envelope<MutationDataDto>, OperationId) {
        let stopped = shelf.repair(REQUEST_1, || shelf.document_repair(MARKER_PATH, source));
        assert_eq!(stopped.outcome, Outcome::Partial);
        let operation = OperationId::parse(stopped.operation_id.as_deref().unwrap()).unwrap();
        (stopped, operation)
    }

    #[test]
    fn a_pending_repair_is_listed_with_its_action() {
        git_home::isolated();
        let shelf = Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let (stopped, _) = interrupted(&shelf, &document(ADOPTED));

        let listed = shelf.service.list_operations(&shelf.resolved()).unwrap();

        assert_eq!(listed.items.len(), 1);
        let pending = &listed.items[0];
        assert_eq!(pending.action, OperationAction::RepairItem);
        assert_eq!(pending.family, OperationFamily::Local);
        assert_eq!(pending.operation_id, stopped.operation_id);
        let value = serde_json::to_value(&listed).unwrap();
        assert_eq!(value["items"][0]["action"], "repair_item");
        schema::check_published("operation_list.schema.json", &value).unwrap();
        // The operation schema's enumeration includes it.
        let published: serde_json::Value = serde_json::from_slice(
            &std::fs::read(schema::schema_directory().join("operation.schema.json")).unwrap(),
        )
        .unwrap();
        assert!(
            published["properties"]["action"]["enum"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("repair_item"))
        );
        // And the journal names it so.
        assert_eq!(
            shelf.count(
                "SELECT COUNT(*) FROM operation_records \
                  WHERE action = 'repair_item' AND state != 'completed'"
            ),
            1
        );
    }

    #[test]
    fn a_resume_of_a_pending_repair_names_the_request_that_goes_on_with_it() {
        git_home::isolated();
        let mut shelf =
            Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let source = document(ADOPTED);
        let (_, operation) = interrupted(&shelf, &source);
        shelf.reopen();

        // The write is made and not committed: the repair needs its
        // source, which only its own request has.
        let resumed = shelf.execute(
            REQUEST_2,
            Mutation::OperationResume(OperationResumeInput {
                root: shelf.root.clone(),
                operation_id: operation,
            }),
        );

        assert_eq!(resumed.code, ResultCode::OriginalRequestRequired);
        assert_eq!(resumed.recovery[0].action.as_str(), "request.retry");
        assert_eq!(
            resumed.recovery[0]
                .arguments
                .get("request_id")
                .and_then(|id| id.as_str()),
            Some(REQUEST_1)
        );
        // Nothing was committed or closed by it.
        assert_eq!(shelf.branch_commits(ADOPTED), 0);
        assert_eq!(shelf.pending_journal_rows(), 1);
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
    }

    #[test]
    fn a_resume_of_a_repair_that_owes_its_hand_off_brings_the_index_up_to_date() {
        git_home::isolated();
        let mut shelf = Shelf::failing_once_at(FailurePoint::BeforeIndexTransactionCommit);
        let source = document(ADOPTED);

        // A failure after the checkpoint is partial.
        let pending = shelf.repair(REQUEST_1, || shelf.document_repair(MARKER_PATH, &source));

        assert_eq!(pending.code, ResultCode::DiscoveryPending);
        assert_eq!(pending.outcome, Outcome::Partial);
        assert_eq!(pending.effects.write, WriteEffect::Written);
        assert_eq!(pending.effects.checkpoint, CheckpointEffect::Committed);
        assert_eq!(pending.effects.discovery, DiscoveryEffect::Pending);
        assert_eq!(
            pending.effects.commit_oid,
            shelf.branch_tip(ADOPTED).map(|tip| tip.to_string())
        );
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        assert_eq!(
            mutation::recovery_before_own_abandon(&pending),
            ["operation.resume"]
        );
        assert_eq!(
            shelf.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        assert_golden(
            "document_repair_discovery_pending",
            &shelf,
            REPAIR_SCHEMA,
            &pending,
        );
        let operation = OperationId::parse(pending.operation_id.as_deref().unwrap()).unwrap();
        shelf.reopen();

        let resumed = shelf.execute(
            REQUEST_2,
            Mutation::OperationResume(OperationResumeInput {
                root: shelf.root.clone(),
                operation_id: operation,
            }),
        );

        assert_eq!(resumed.code, ResultCode::Ok);
        assert_eq!(resumed.effects.discovery, DiscoveryEffect::Current);
        assert_eq!(shelf.pending_journal_rows(), 0);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
        let shown = shelf
            .service
            .show_item(&shelf.resolved(), &items::item_id(ADOPTED))
            .unwrap();
        assert_eq!(shown.source.as_deref(), Some(source.as_str()));
    }

    #[test]
    fn an_abandoned_repair_leaves_its_written_file_and_frees_its_request() {
        git_home::isolated();
        let mut shelf =
            Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let source = document(ADOPTED);
        let (_, operation) = interrupted(&shelf, &source);
        shelf.reopen();
        let abandon = || {
            Mutation::OperationAbandon(OperationAbandonInput {
                root: shelf.root.clone(),
                operation_id: operation,
            })
        };

        let (confirmation, preview) = shelf.preview(&abandon());

        // What the abandon says the repair left is what it left.
        let PreviewDetailsDto::OperationAbandon(details) = &preview.details else {
            panic!("not the preview of an abandon");
        };
        let worktree = format!(".manyhands/worktrees/{ADOPTED}");
        assert_eq!(details.action, Some(OperationAction::RepairItem));
        assert_eq!(details.item_id.as_deref(), Some(ADOPTED));
        assert_eq!(details.worktree.as_deref(), Some(worktree.as_str()));
        assert_eq!(details.paths, [format!("{worktree}/{MARKER_PATH}")]);
        assert!(details.paths_complete);
        assert!(!details.committed);
        assert_eq!(details.request_id.as_deref(), Some(REQUEST_1));

        let abandoned = shelf.confirmed(REQUEST_2, &confirmation, abandon());

        assert_eq!(abandoned.code, ResultCode::Ok);
        let data = mutation::operation_data(&abandoned);
        assert_eq!(data.action, Some(OperationAction::RepairItem));
        assert_eq!(data.item_id.as_deref(), Some(ADOPTED));
        assert_eq!(data.paths, [format!("{worktree}/{MARKER_PATH}")]);
        assert!(!data.committed);
        assert_eq!(data.released_request_id.as_deref(), Some(REQUEST_1));
        // The written source is left where it was, not committed.
        assert_eq!(
            shelf.context(ADOPTED, MARKER_PATH).as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(shelf.branch_commits(ADOPTED), 0);
        assert_eq!(shelf.pending_journal_rows(), 0);
        assert!(shelf.record(REQUEST_1).is_none());

        // The freed request, sent again with a new preview, is a new
        // request: it commits what was left, under the same ID.
        let done = shelf.repair(REQUEST_1, || shelf.document_repair(MARKER_PATH, &source));
        assert_eq!(done.code, ResultCode::Ok);
        assert_ne!(
            done.operation_id.as_deref(),
            Some(operation.to_string().as_str())
        );
        assert_eq!(repair_data(&done).id, ADOPTED);
        assert_eq!(shelf.contexts(), [ADOPTED]);
        assert_eq!(shelf.branch_commits(ADOPTED), 1);
    }

    #[test]
    fn a_refresh_and_a_rebuild_never_adopt() {
        git_home::isolated();
        let shelf = Shelf::new();
        let paths = [
            MARKER_PATH.to_owned(),
            UNTITLED_PATH.to_owned(),
            OUTSIDE_PATH.to_owned(),
            items::ticket_path(TICKET_MARKER),
            TICKET_UNNAMED_PATH.to_owned(),
            items::ticket_path(TICKET_UNREADABLE),
        ];
        let files = |shelf: &Shelf| {
            paths
                .iter()
                .map(|path| shelf.primary(path))
                .collect::<Vec<_>>()
        };
        let before = files(&shelf);
        let head = shelf.primary_head();

        items::refresh_completely(&shelf.service, &shelf.root);
        items::rebuild(&shelf.service, &shelf.root);

        // Byte for byte as they were, and still no item.
        assert_eq!(files(&shelf), before);
        assert_eq!(before[0], marker_only().into_bytes());
        assert_eq!(shelf.primary_head(), head);
        assert!(shelf.contexts().is_empty());
        assert!(shelf.item_branches().is_empty());
        let listed = shelf.service.list_documents(&shelf.resolved()).unwrap();
        let marker: Vec<_> = listed
            .items
            .iter()
            .filter(|item| item.path == MARKER_PATH)
            .collect();
        assert_eq!(marker.len(), 1);
        assert_eq!(marker[0].id, None);
    }

    #[test]
    fn nothing_a_repair_reads_reaches_what_it_gives_or_keeps() {
        git_home::isolated();
        let mut shelf =
            Shelf::failing_once_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
        let source = document(ADOPTED);
        let token = shelf.path_token(MARKER_PATH);
        let repair = |shelf: &Shelf| shelf.document_repair_with(MARKER_PATH, &token, &source);
        // The command reads both: the file it is asked to repair, and
        // the source it is given.
        assert!(String::from_utf8_lossy(&shelf.primary(MARKER_PATH)).contains(FILE_SENTINEL));
        assert!(source.contains(SOURCE_SENTINEL));

        // A preview, a request stopped part of the way, a refusal, and
        // the request completed. `Shelf` scans every envelope it gives.
        let (confirmation, _) = shelf.preview(&repair(&shelf));
        let stopped = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(stopped.outcome, Outcome::Partial);
        // While the record is accepted and its row pending.
        assert_eq!(
            mutation::find_stored_sentinel(shelf.data.path(), &SENTINELS),
            None
        );
        let record = format!("{:?}", shelf.record(REQUEST_1).unwrap());
        for sentinel in SENTINELS {
            assert!(!record.contains(sentinel));
        }
        let mismatch = shelf.confirmed(
            REQUEST_1,
            &confirmation,
            shelf.document_repair_with(MARKER_PATH, &token, &document(UNHELD)),
        );
        assert_eq!(mismatch.code, ResultCode::RequestMismatch);
        shelf.reopen();
        let done = shelf.confirmed(REQUEST_1, &confirmation, repair(&shelf));
        assert_eq!(done.code, ResultCode::Ok);

        // The source went where it was to go, and nowhere else.
        assert!(
            String::from_utf8_lossy(&shelf.context(ADOPTED, MARKER_PATH).unwrap())
                .contains(SOURCE_SENTINEL)
        );
        assert_eq!(
            mutation::find_stored_sentinel(shelf.data.path(), &SENTINELS),
            None
        );
        let shown = serde_json::to_string(
            &shelf
                .service
                .show_request(support::records::request_id(REQUEST_1)),
        )
        .unwrap();
        let operations =
            serde_json::to_string(&shelf.service.list_operations(&shelf.resolved()).unwrap())
                .unwrap();
        for sentinel in SENTINELS {
            assert!(!shown.contains(sentinel), "request show holds {sentinel}");
            assert!(
                !operations.contains(sentinel),
                "the operations hold {sentinel}"
            );
        }
        // The scan can fail: the same scan finds a sentinel that is there.
        items::index(shelf.data.path())
            .execute(
                "UPDATE operation_records SET redacted_error = ?1",
                [FILE_SENTINEL],
            )
            .unwrap();
        assert!(mutation::find_stored_sentinel(shelf.data.path(), &SENTINELS).is_some());
        items::index(shelf.data.path())
            .execute("UPDATE operation_records SET redacted_error = NULL", [])
            .unwrap();
    }
}
