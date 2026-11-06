//! The conflict model, continued: a body taken from one side, a remote
//! document path that does not classify, and `historical` (Task 10
//! review). Canned as the parent module's tests are, citing the same
//! library tests.

use manyhands::repository::{
    ConflictPathKind, ConflictPathStructure, ConflictReadEligibility, LocalContextSyncEligibility,
    LocalContextSyncReason,
};

use manyhands::canonical::CanonicalItem;

use super::{
    canonical, content, entry, external, inspection, path, remote, ticket, ticket_conflict,
};
use crate::desktop::{
    model::conflict::{BodyChoice, Choice, ConflictModel, ExternalReason, PathResult},
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

/// What the library requires of a result against each recorded side
/// (`integration::validate::preserves_item_invariants`, which is private to
/// the library): a closed ticket keeps its closure, and a comment keeps its
/// item, parent and creation as they are.
fn passes_library(side: &CanonicalItem, result: &CanonicalItem) -> bool {
    match (side, result) {
        (CanonicalItem::Ticket(old), CanonicalItem::Ticket(new)) => {
            old.closed_at.is_none()
                || (old.closed_at == new.closed_at && old.closed_by == new.closed_by)
        }
        (CanonicalItem::Comment(old), CanonicalItem::Comment(new)) => {
            old.item_id == new.item_id
                && old.parent_id == new.parent_id
                && old.created_at == new.created_at
                && old.created_by == new.created_by
                && old.unknown.get("created_by") == new.unknown.get("created_by")
                && old.unknown.get("author") == new.unknown.get("author")
        }
        _ => true,
    }
}

fn closed_ticket(title: &str, by: Option<&str>) -> String {
    let by = by.map_or(String::new(), |by| format!("closed_by: {by}\n"));
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {}\ntitle: {title}\n\
         type: task\nstatus: done\nclosed_at: 2026-10-25T12:00:00Z\n{by}---\nBody.\n",
        super::TICKET
    )
}

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ0";
const OTHER_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";

fn comment(parent: Option<&str>, author: &str, body: &str) -> String {
    let parent = parent.map_or(String::new(), |parent| format!("parent_id: {parent}\n"));
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {COMMENT}\n\
         item_id: {}\n{parent}created_at: 2026-09-30T12:00:00Z\ncreated_by: {author}\n---\n{body}",
        super::TICKET
    )
}

/// V11, canned: a one-sided take of a field the library will not let a
/// result change gave a Result it refused (`InvalidInput`), so every save
/// was refused. A closed ticket's closure stays in the result however the
/// sides took it (also when both sides reopened it), and a comment whose
/// sides disagree on an immutable field has no result the library takes,
/// so it goes outside.
#[test]
fn a_result_keeps_what_the_library_will_not_let_change() {
    let one = Some("One");
    let ticket_case = |base: String, mine: String, theirs: String| {
        (
            ticket_path(super::TICKET),
            ConflictPathKind::Ticket,
            (base, mine, theirs),
        )
    };
    let open_ticket = |title: &str| ticket(title, "open", "Body.\n");
    let comment_path = format!(".manyhands/comments/{}/{COMMENT}.md", super::TICKET);
    let comment_case = |base: String, mine: String, theirs: String| {
        (
            comment_path.clone(),
            ConflictPathKind::Comment,
            (base, mine, theirs),
        )
    };
    // (name, case, whether a result can pass)
    let cases = vec![
        (
            "theirs reopened it, mine kept it closed",
            ticket_case(
                closed_ticket("Base", one),
                closed_ticket("Local", one),
                open_ticket("Base"),
            ),
            true,
        ),
        (
            "mine reopened it, theirs kept it closed",
            ticket_case(
                closed_ticket("Base", one),
                open_ticket("Local"),
                closed_ticket("Incoming", one),
            ),
            true,
        ),
        (
            "both reopened it",
            ticket_case(
                closed_ticket("Base", one),
                open_ticket("Local"),
                open_ticket("Incoming"),
            ),
            true,
        ),
        (
            "only theirs closed it",
            ticket_case(
                open_ticket("Base"),
                open_ticket("Local"),
                closed_ticket("Incoming", one),
            ),
            true,
        ),
        (
            "only mine closed it",
            ticket_case(
                open_ticket("Base"),
                closed_ticket("Local", one),
                open_ticket("Incoming"),
            ),
            true,
        ),
        (
            "theirs gave the comment another parent",
            comment_case(
                comment(None, "Ada", "Base\n"),
                comment(None, "Ada", "Mine\n"),
                comment(Some(OTHER_COMMENT), "Ada", "Theirs\n"),
            ),
            false,
        ),
        (
            "theirs changed who made the comment",
            comment_case(
                comment(None, "Ada", "Base\n"),
                comment(None, "Ada", "Mine\n"),
                comment(None, "Grace", "Theirs\n"),
            ),
            false,
        ),
        (
            "the comment's creation is the same, its body differs",
            comment_case(
                comment(None, "Ada", "Base\n"),
                comment(None, "Ada", "Mine\n"),
                comment(None, "Ada", "Theirs\n"),
            ),
            true,
        ),
    ];
    for (name, (display_path, kind, (base, mine, theirs)), passes) in cases {
        let mut model = ConflictModel::from_remote(&remote(
            vec![path(
                &display_path,
                kind,
                ConflictPathStructure::Regular,
                Some((base.clone(), mine.clone(), theirs.clone())),
            )],
            ConflictReadEligibility::EligibleCanonical,
        ));
        if !passes {
            assert_eq!(
                external(&model),
                [(display_path, ExternalReason::Unreadable)],
                "{name}: no result passes, so it goes outside"
            );
            continue;
        }
        let sides: Vec<_> = [&base, &mine, &theirs]
            .into_iter()
            .map(|source| {
                manyhands::canonical::parse_item(std::path::Path::new(&display_path), source)
                    .unwrap()
            })
            .collect();
        let path = &mut model.paths_mut()[0];
        for index in 0..path.fields().len() {
            path.choose_field(index, Choice::Theirs);
        }
        if path.body().is_some() {
            path.choose_body(BodyChoice::Theirs);
        }
        let PathResult::Ready { source, .. } = path.result() else {
            panic!("{name}: every part is chosen");
        };
        let result =
            manyhands::canonical::parse_item(std::path::Path::new(&display_path), &source).unwrap();
        for side in &sides {
            assert!(
                passes_library(side, &result),
                "{name}: the result must keep what the library will not let change"
            );
        }
    }
}
