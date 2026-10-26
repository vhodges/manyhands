//! Two-phase confirmation, through `document repair`: a preview that is
//! stale by the time its confirmation is presented, and a confirmation
//! presented with another input than the one it was given for.

use super::{
    support::{mutation::recovery_actions, repair::*},
    *,
};

/// What someone commits on primary over the file a preview was of.
const EDITED: &str = "---\nmanyhands_managed: true\n---\nEdited since the preview.\n";

/// Asserts that nothing is left of a request that was refused: no
/// record, no confirmation held, no operation in flight, no context and
/// no branch.
fn assert_nothing_left(shelf: &Shelf, request_id: &str) {
    assert!(shelf.record(request_id).is_none(), "a request record");
    assert_eq!(shelf.request_rows(), 0, "request records");
    assert_eq!(
        shelf.count(
            "SELECT COUNT(*) FROM confirmation_records WHERE accepted_by_request IS NOT NULL"
        ),
        0,
        "a confirmation held"
    );
    assert_eq!(shelf.pending_journal_rows(), 0, "an operation in flight");
    assert!(shelf.contexts().is_empty(), "an editing context");
    assert!(shelf.item_branches().is_empty(), "an item branch");
}

#[test]
fn a_file_changed_and_committed_on_primary_after_the_preview_is_an_external_change() {
    let shelf = Shelf::new();
    let source = document(ADOPTED);
    let token = shelf.path_token(MARKER_PATH);
    let repair = || shelf.document_repair_with(MARKER_PATH, &token, &source);
    let (confirmation, _) = shelf.preview(&repair());
    let rows = shelf.journal_rows();

    shelf.commit_on_primary(MARKER_PATH, EDITED);

    // The request is the one the confirmation was given for: the same
    // command, repository, path, token and source. It passes every check
    // of the confirmation itself, and is stopped by the binding's own
    // observation, which is no longer the one that was previewed.
    let refused = shelf.confirmed(REQUEST_1, &confirmation, repair());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.effects, Effects::not_requested());
    // A new preview shows what is there now.
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert_nothing_left(&shelf, REQUEST_1);
    assert_eq!(shelf.journal_rows(), rows);
    assert_eq!(shelf.primary(MARKER_PATH), EDITED.as_bytes());
    let root = shelf.root.to_str().unwrap().to_owned();
    mutation::assert_contract(
        &golden::ContractCase {
            name: "document_repair_external_change",
            data_schema: refused.data.as_ref().map(|_| "repair_mutation.schema.json"),
            placeholders: &[(root.as_str(), "<root>")],
            sentinels: &SENTINELS,
        },
        &refused,
    );

    // The confirmation was not used up by the refusal, and is still of
    // a file that is no longer there: the same request is refused again.
    let again = shelf.confirmed(REQUEST_1, &confirmation, repair());
    assert_eq!(again.code, ResultCode::ExternalChange);

    // With the file as it was previewed, the same confirmation runs.
    shelf.commit_on_primary(MARKER_PATH, &marker_only());
    let done = shelf.confirmed(REQUEST_1, &confirmation, repair());
    assert_eq!(done.code, ResultCode::Ok);
    assert_eq!(repair_data(&done).id, ADOPTED);
}

#[test]
fn a_file_edited_and_not_committed_after_the_preview_is_an_external_change() {
    let shelf = Shelf::new();
    let source = document(ADOPTED);
    let token = shelf.path_token(MARKER_PATH);
    let repair = || shelf.document_repair_with(MARKER_PATH, &token, &source);
    let (confirmation, _) = shelf.preview(&repair());

    items::write(&shelf.root, MARKER_PATH, EDITED);

    let refused = shelf.confirmed(REQUEST_1, &confirmation, repair());

    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_nothing_left(&shelf, REQUEST_1);
    assert_eq!(shelf.primary(MARKER_PATH), EDITED.as_bytes());
}

#[test]
fn another_source_under_the_same_confirmation_is_a_confirmation_mismatch() {
    let shelf = Shelf::new();
    let token = shelf.path_token(MARKER_PATH);
    let repair = |source: &str| shelf.document_repair_with(MARKER_PATH, &token, source);
    let consented = document(ADOPTED);
    let (confirmation, _) = shelf.preview(&repair(&consented));
    let rows = shelf.journal_rows();

    // Another ID, and the same ID with another body.
    for source in [document(UNHELD), format!("{consented}More.\n")] {
        let refused = shelf.confirmed(REQUEST_1, &confirmation, repair(&source));

        assert_eq!(refused.code, ResultCode::ConfirmationMismatch);
        assert_eq!(refused.outcome, Outcome::Error);
        assert!(recovery_actions(&refused).is_empty());
        assert_nothing_left(&shelf, REQUEST_1);
        assert_eq!(shelf.journal_rows(), rows);
    }
    // So is the same source for another path, or with another token.
    let elsewhere = shelf.confirmed(
        REQUEST_1,
        &confirmation,
        shelf.document_repair_with(UNTITLED_PATH, &token, &consented),
    );
    assert_eq!(elsewhere.code, ResultCode::ConfirmationMismatch);
    let other_token = shelf.confirmed(
        REQUEST_1,
        &confirmation,
        shelf.document_repair_with(MARKER_PATH, "v1:other", &consented),
    );
    assert_eq!(other_token.code, ResultCode::ConfirmationMismatch);
    assert_nothing_left(&shelf, REQUEST_1);

    // The confirmation is still the consented request's to use.
    let done = shelf.confirmed(REQUEST_1, &confirmation, repair(&consented));
    assert_eq!(done.code, ResultCode::Ok);
    assert_eq!(
        shelf.context(ADOPTED, MARKER_PATH).as_deref(),
        Some(consented.as_bytes())
    );
}

#[test]
fn a_repair_is_not_run_without_its_confirmation() {
    let shelf = Shelf::new();
    let source = document(ADOPTED);
    let repair = || shelf.document_repair(MARKER_PATH, &source);

    let unconfirmed = execute_with(&shelf.service, REQUEST_1, repair());

    assert_eq!(unconfirmed.code, ResultCode::ConfirmationRequired);
    assert_eq!(recovery_actions(&unconfirmed), ["request.prepare"]);
    assert_nothing_left(&shelf, REQUEST_1);

    // A confirmation is used by one request.
    let (confirmation, _) = shelf.preview(&repair());
    let done = shelf.confirmed(REQUEST_1, &confirmation, repair());
    assert_eq!(done.code, ResultCode::Ok);
    let second = shelf.confirmed(REQUEST_2, &confirmation, repair());
    assert_eq!(second.code, ResultCode::ConfirmationUsed);
    assert!(shelf.record(REQUEST_2).is_none());
}
