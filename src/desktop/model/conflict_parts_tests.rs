//! The conflict model, continued: a body taken from one side, a remote
//! document path that does not classify, and `historical` (Task 10
//! review). Canned as the parent module's tests are, citing the same
//! library tests.

use manyhands::repository::{
    ConflictPathKind, ConflictPathStructure, ConflictReadEligibility, LocalContextSyncEligibility,
    LocalContextSyncReason,
};

use super::{
    canonical, content, entry, external, inspection, path, remote, ticket, ticket_conflict,
};
use crate::desktop::{
    model::conflict::{Choice, ConflictModel, ExternalReason},
    test_support::fixtures::{Fixture, ticket_path},
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG2";

fn document(title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\n\
         title: {title}\n---\nBody.\n"
    )
}

/// Canned, as `c4_conflict_read_canonical`: a body only one side changed
/// is taken from it (R-5), and says which side, for the columns; a body
/// neither changed says none.
#[test]
fn a_body_changed_by_one_side_is_taken() {
    let model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Base body.\n"),
            ticket("Local", "open", "Base body.\n"),
            ticket("Incoming", "open", "Theirs body.\n"),
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    let paths = canonical(&model);
    assert!(paths[0].body().is_none());
    assert_eq!(paths[0].body_taken(), Some(Choice::Theirs));

    let model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Body.\n"),
            ticket("Local", "open", "Body.\n"),
            ticket("Incoming", "open", "Body.\n"),
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    assert_eq!(canonical(&model)[0].body_taken(), None);
}

/// K-14, widened to remote conflicts (owner): a remote document whose
/// path has a non-ASCII byte, escaped in its display path, does not
/// classify, and goes outside as the local case does. A document whose
/// path classifies and whose side does not parse is still unreadable.
/// Canned, as `c4_conflict_read_canonical`.
#[test]
fn a_non_ascii_remote_document_goes_outside_as_unsupported() {
    let escaped = "docs/caf\\xc3\\xa9.md";
    let sides = Some((document("Base"), document("Local"), document("Incoming")));
    let model = ConflictModel::from_remote(&remote(
        vec![path(
            escaped,
            ConflictPathKind::Document,
            ConflictPathStructure::Regular,
            sides,
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    assert_eq!(
        external(&model),
        [(escaped.to_owned(), ExternalReason::Unsupported)]
    );

    let broken = Some((
        document("Base"),
        "not an item".to_owned(),
        document("Incoming"),
    ));
    let model = ConflictModel::from_remote(&remote(
        vec![path(
            "docs/plain.md",
            ConflictPathKind::Document,
            ConflictPathStructure::Regular,
            broken,
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    assert_eq!(
        external(&model),
        [("docs/plain.md".to_owned(), ExternalReason::Unreadable)]
    );
}

/// `historical` is carried from the read for the sheet: the remote
/// conflict's, and the local content's. A real local conflict with no
/// attempt is not historical.
#[test]
fn historical_is_carried_from_the_read() {
    let mut conflict = remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Body.\n"),
            ticket("Local", "open", "Body.\n"),
            ticket("Incoming", "open", "Body.\n"),
        )],
        ConflictReadEligibility::EligibleCanonical,
    );
    assert!(!ConflictModel::from_remote(&conflict).historical());
    conflict.historical = true;
    assert!(ConflictModel::from_remote(&conflict).historical());

    let ticket_entry = entry(&ticket_path(super::TICKET), true);
    let model = ConflictModel::from_local(
        &inspection(
            LocalContextSyncReason::CanonicalConflict,
            LocalContextSyncEligibility::EligibleCanonical,
            vec![ticket_entry.clone()],
        ),
        &[content(&ticket_entry.token, true)],
    );
    assert!(model.historical());

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = fixture.conflict_locally(&root, super::TICKET, "Mine", "Theirs");
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    assert!(!ConflictModel::from_local(&inspection, &contents).historical());
}
