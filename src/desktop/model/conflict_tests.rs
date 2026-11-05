//! The conflict model (D5 design, B2).
//!
//! The remote conflicts here are canned `ConflictDto`s, built from the
//! library's types: no desktop test can reach a remote synchronization's
//! conflict. Each test names the library test whose read produces the real
//! answer of its shape. The local conflicts are real, but for two shapes no
//! local fixture reaches, which name theirs too.

use std::path::Path;

use manyhands::{
    canonical::{CanonicalItem, parse_item, serialize_item},
    repository::{
        ConflictDto, ConflictPathDto, ConflictPathKind, ConflictPathStructure,
        ConflictReadEligibility, ConflictReadNextAction, ConflictStage, ConflictState,
        ConflictTarget, LocalContextConflictContentDto, LocalContextConflictEntryDto,
        LocalContextSyncEligibility, LocalContextSyncInspectionDto, LocalContextSyncNextAction,
        LocalContextSyncReason, LocalContextSyncState,
    },
};

use super::{
    BodyChoice, Choice, ConflictModel, ConflictSet, ExternalReason, PathResult,
    STANDARD_LAYOUT_NOTE,
};
use crate::desktop::test_support::fixtures::{Fixture, ticket_path};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH0";

/// A ticket's source, in the layout `tests/mutation_remote/conflict_read.rs`
/// commits (`source`).
fn ticket(title: &str, status: &str, body: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\n\
         title: {title}\ntype: task\nstatus: {status}\n---\n{body}"
    )
}

fn path(
    display_path: &str,
    kind: ConflictPathKind,
    structure: ConflictPathStructure,
    sides: Option<(String, String, String)>,
) -> ConflictPathDto {
    let content_available = sides.is_some();
    let (base, local, incoming) = match sides {
        Some((base, local, incoming)) => (Some(base), Some(local), Some(incoming)),
        None => (None, None, None),
    };
    ConflictPathDto {
        display_path: display_path.to_owned(),
        token: format!("conflict-path-v1:{display_path}"),
        kind,
        structure,
        content_available,
        base,
        local,
        incoming,
    }
}

fn ticket_conflict(base: String, mine: String, theirs: String) -> ConflictPathDto {
    path(
        &ticket_path(TICKET),
        ConflictPathKind::Ticket,
        ConflictPathStructure::Regular,
        Some((base, mine, theirs)),
    )
}

/// A canned remote conflict of the shape `tests/mutation_remote/
/// conflict_read.rs` `c4_conflict_read_canonical` reads.
fn remote(paths: Vec<ConflictPathDto>, eligibility: ConflictReadEligibility) -> ConflictDto {
    ConflictDto {
        operation_id: OPERATION.to_owned(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state: ConflictState::ConflictPending,
        next_action: match eligibility {
            ConflictReadEligibility::EligibleCanonical => ConflictReadNextAction::Resolve,
            ConflictReadEligibility::ExternalResolutionRequired => {
                ConflictReadNextAction::ExternalRepair
            }
        },
        request_id: None,
        eligibility: Some(eligibility),
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: Some(format!("conflict-v1:{}", "0".repeat(64))),
        historical: false,
        paths,
    }
}

fn canonical(model: &ConflictModel) -> &[super::PathConflict] {
    match model.set() {
        ConflictSet::Canonical(paths) => paths,
        _ => panic!("a canonical set"),
    }
}

fn external(model: &ConflictModel) -> Vec<(String, ExternalReason)> {
    match model.set() {
        ConflictSet::External(paths) => paths
            .iter()
            .map(|path| (path.display_path.clone(), path.reason))
            .collect(),
        _ => panic!("an external set"),
    }
}

fn parsed(source: &str) -> CanonicalItem {
    parse_item(Path::new(&ticket_path(TICKET)), source).unwrap()
}

/// Canned, as `c4_conflict_read_canonical`: both sides retitled the
/// ticket, differently, so the title is a part with both values, and
/// nothing can be saved until it is chosen.
#[test]
fn a_field_changed_by_both_needs_a_choice() {
    let model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Body.\n"),
            ticket("Local", "open", "Body.\n"),
            ticket("Incoming", "open", "Body.\n"),
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    let paths = canonical(&model);
    assert_eq!(paths.len(), 1);
    let fields = paths[0].fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].key, "title");
    assert_eq!(fields[0].base.as_deref(), Some("Base"));
    assert_eq!(fields[0].mine.as_deref(), Some("Local"));
    assert_eq!(fields[0].theirs.as_deref(), Some("Incoming"));
    assert_eq!(fields[0].choice, None);
    assert!(paths[0].body().is_none(), "neither changed the body");
    assert!(matches!(paths[0].result(), PathResult::Pending));
    assert_eq!(model.chosen(), (0, 1));
    assert!(model.resolutions().is_none());
    assert_eq!(
        model.observation(),
        Some(format!("conflict-v1:{}", "0".repeat(64)).as_str())
    );
}

/// Canned, as `c4_conflict_read_canonical`: a field only one side changed
/// is taken from it with no choice (R-5), and still listed; with nothing
/// left to choose the result is ready.
#[test]
fn a_field_changed_by_one_side_is_taken() {
    let base = ticket("Base", "open", "Body.\n");
    let mine = ticket("Local", "open", "Body.\n");
    let theirs = ticket("Base", "done", "Body.\n");
    let model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(base, mine, theirs)],
        ConflictReadEligibility::EligibleCanonical,
    ));
    let paths = canonical(&model);
    assert!(paths[0].fields().is_empty());
    let taken: Vec<(&str, Choice)> = paths[0]
        .taken()
        .iter()
        .map(|field| (field.key.as_str(), field.from))
        .collect();
    assert_eq!(taken, [("title", Choice::Mine), ("status", Choice::Theirs)]);
    assert_eq!(model.chosen(), (0, 0));
    let PathResult::Ready { source, normalized } = paths[0].result() else {
        panic!("nothing to choose");
    };
    assert!(normalized, "theirs' status is taken into mine");
    let CanonicalItem::Ticket(result) = parsed(&source) else {
        panic!("a ticket");
    };
    assert_eq!(result.title, "Local");
    assert_eq!(result.status, "done");
}

/// Canned, as `c4_conflict_read_canonical`: a body both changed is one
/// part, whose Keep both is mine, a blank line, then theirs (R-4); the
/// result stays editable.
#[test]
fn the_body_offers_keep_both() {
    let mut model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Base body.\n"),
            ticket("Base", "open", "Mine body.\n"),
            ticket("Base", "open", "Theirs body.\n"),
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    let path = &mut model.paths_mut()[0];
    let body = path.body().expect("both changed the body");
    assert_eq!(body.base, "Base body.\n");
    assert_eq!(body.mine, "Mine body.\n");
    assert_eq!(body.theirs, "Theirs body.\n");
    assert_eq!(body.choice, None);
    assert!(matches!(path.result(), PathResult::Pending));

    path.choose_body(BodyChoice::Theirs);
    assert_eq!(path.body().unwrap().result, "Theirs body.\n");
    path.choose_body(BodyChoice::Both);
    assert_eq!(path.body().unwrap().result, "Mine body.\n\nTheirs body.\n");
    let PathResult::Ready { source, .. } = path.result() else {
        panic!("every part is chosen");
    };
    let CanonicalItem::Ticket(result) = parsed(&source) else {
        panic!("a ticket");
    };
    assert_eq!(result.body, "Mine body.\n\nTheirs body.\n");

    path.edit_body("Edited by hand.\n".to_owned());
    assert_eq!(path.body().unwrap().choice, Some(BodyChoice::Edited));
    let PathResult::Ready { source, .. } = path.result() else {
        panic!("an edited result is chosen");
    };
    assert!(source.ends_with("---\nEdited by hand.\n"));
    assert_eq!(model.chosen(), (1, 1));
}

/// Canned, as `c4_conflict_read_canonical`: when every part is Keep mine
/// and nothing was taken from theirs, mine's bytes are submitted as they
/// are, layout and all (R-9).
#[test]
fn all_mine_submits_mines_bytes() {
    // Mine's own layout: a key Manyhands does not define, first.
    let mine = format!(
        "---\nextra: kept\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\n\
         title: Local\ntype: task\nstatus: open\n---\nMine body.\n"
    );
    let mut model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Base body.\n"),
            mine.clone(),
            ticket("Incoming", "open", "Theirs body.\n"),
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    let path = &mut model.paths_mut()[0];
    // The key only mine added is taken from mine.
    assert!(path.taken().iter().all(|field| field.from == Choice::Mine));
    path.choose_field(0, Choice::Mine);
    path.choose_body(BodyChoice::Mine);
    let PathResult::Ready { source, normalized } = path.result() else {
        panic!("every part is chosen");
    };
    assert_eq!(source, mine);
    assert!(!normalized);
    let resolutions = model.resolutions().expect("every part is chosen");
    assert_eq!(resolutions.len(), 1);
    assert_eq!(
        resolutions[0].token,
        format!("conflict-path-v1:{}", ticket_path(TICKET))
    );
    assert_eq!(resolutions[0].source, mine);
}

/// Canned, as `c4_conflict_read_canonical`: a chosen merge is the item
/// with the chosen values, written in the standard layout, with the card's
/// note (R-9).
#[test]
fn a_chosen_merge_submits_a_normalized_file() {
    let mine = ticket("Local", "open", "Mine body.\n");
    let theirs = ticket("Incoming", "open", "Theirs body.\n");
    let mut model = ConflictModel::from_remote(&remote(
        vec![ticket_conflict(
            ticket("Base", "open", "Base body.\n"),
            mine.clone(),
            theirs,
        )],
        ConflictReadEligibility::EligibleCanonical,
    ));
    let path = &mut model.paths_mut()[0];
    path.choose_field(0, Choice::Theirs);
    path.choose_body(BodyChoice::Mine);
    let PathResult::Ready { source, normalized } = path.result() else {
        panic!("every part is chosen");
    };
    assert!(normalized);
    let mut expected = parsed(&mine);
    let CanonicalItem::Ticket(ticket) = &mut expected else {
        panic!("a ticket");
    };
    ticket.title = "Incoming".to_owned();
    assert_eq!(source, serialize_item(&expected).unwrap());
    assert_eq!(
        STANDARD_LAYOUT_NOTE,
        "This file will be saved in the standard layout."
    );
    assert_eq!(model.resolutions().unwrap()[0].source, source);
}

/// Canned, as `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_noncanonical_or_structural`: one noncanonical path
/// sends the whole set outside, the canonical one with it (R-6).
#[test]
fn a_noncanonical_path_makes_the_set_external() {
    let model = ConflictModel::from_remote(&remote(
        vec![
            ticket_conflict(
                ticket("Base", "open", "Body.\n"),
                ticket("Local", "open", "Body.\n"),
                ticket("Incoming", "open", "Body.\n"),
            ),
            path(
                "blob.bin",
                ConflictPathKind::Noncanonical,
                ConflictPathStructure::Binary,
                None,
            ),
            path(
                "plain.sh",
                ConflictPathKind::Noncanonical,
                ConflictPathStructure::Binary,
                None,
            ),
            path(
                "tool.sh",
                ConflictPathKind::Noncanonical,
                ConflictPathStructure::Executable,
                None,
            ),
        ],
        ConflictReadEligibility::ExternalResolutionRequired,
    ));
    assert_eq!(
        external(&model),
        [
            (ticket_path(TICKET), ExternalReason::Waits),
            ("blob.bin".to_owned(), ExternalReason::Code),
            ("plain.sh".to_owned(), ExternalReason::Code),
            ("tool.sh".to_owned(), ExternalReason::Code),
        ]
    );
    assert_eq!(model.chosen(), (0, 0));
    assert!(model.resolutions().is_none());
}

/// Canned, as `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_incompatible_closures`: two different closures have
/// no winner here, so the set goes outside.
#[test]
fn incompatible_closures_are_external() {
    let closed = |by: &str| {
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\ntitle: Base\n\
             type: task\nstatus: done\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: {by}\n---\nBody.\n"
        )
    };
    let model = ConflictModel::from_remote(&remote(
        vec![path(
            &ticket_path(TICKET),
            ConflictPathKind::Ticket,
            ConflictPathStructure::IncompatibleClosures,
            Some((
                ticket("Base", "open", "Body.\n"),
                closed("One"),
                closed("Two"),
            )),
        )],
        ConflictReadEligibility::ExternalResolutionRequired,
    ));
    assert_eq!(
        external(&model),
        [(ticket_path(TICKET), ExternalReason::Closures)]
    );
}

/// A local inspection of the shape `tests/cli/local_sync.rs`
/// `an_eligible_conflict_stops_with_its_operation_and_the_way_on` reads.
fn inspection(
    reason: LocalContextSyncReason,
    eligibility: LocalContextSyncEligibility,
    paths: Vec<LocalContextConflictEntryDto>,
) -> LocalContextSyncInspectionDto {
    LocalContextSyncInspectionDto {
        operation_id: OPERATION.to_owned(),
        kind: "ticket".to_owned(),
        item_id: TICKET.to_owned(),
        context_ref: format!("refs/heads/manyhands/ticket/{TICKET}"),
        primary_ref: "refs/heads/main".to_owned(),
        context_parent: "1".repeat(40),
        primary_parent: "2".repeat(40),
        context_head: Some("1".repeat(40)),
        state: LocalContextSyncState::ConflictPending,
        reason,
        next_action: match eligibility {
            LocalContextSyncEligibility::EligibleCanonical => LocalContextSyncNextAction::Resolve,
            LocalContextSyncEligibility::ExternalResolutionRequired => {
                LocalContextSyncNextAction::ExternalRepair
            }
        },
        observation: Some(format!("local-sync-v1:{}", "0".repeat(64))),
        eligibility: Some(eligibility),
        paths,
        can_start_resolution: true,
        starting_request_id: OTHER.to_owned(),
        cancellation_request_id: OTHER.to_owned(),
        active_resolution_request_id: None,
        required_request_id: None,
        result_oid: None,
        cancel_requested: false,
        cancel_acknowledged: false,
    }
}

fn entry(display_path: &str, content_available: bool) -> LocalContextConflictEntryDto {
    LocalContextConflictEntryDto {
        display_path: display_path.to_owned(),
        token: format!("token-{display_path}"),
        content_available,
    }
}

/// Canned, as `src/repository/local_sync/task6_quality_tests.rs`
/// `quality_representable_unsupported_conflicts_remain_external_only`: a
/// local set the library cannot resolve goes outside, whatever its paths.
#[test]
fn a_local_unsupported_reason_is_external() {
    // A ticket whose sides parse, its eligibility left eligible, so the
    // reason alone decides.
    let ticket_entry = entry(&ticket_path(TICKET), true);
    let contents = [content(&ticket_entry.token, false)];
    let model = ConflictModel::from_local(
        &inspection(
            LocalContextSyncReason::UnsupportedConflict,
            LocalContextSyncEligibility::EligibleCanonical,
            vec![ticket_entry.clone()],
        ),
        &contents,
    );
    assert_eq!(
        external(&model),
        [(ticket_path(TICKET), ExternalReason::Unsupported)]
    );

    // An immutable closure disagreement says so.
    let model = ConflictModel::from_local(
        &inspection(
            LocalContextSyncReason::ImmutableClosureDisagreement,
            LocalContextSyncEligibility::EligibleCanonical,
            vec![ticket_entry],
        ),
        &contents,
    );
    assert_eq!(
        external(&model),
        [(ticket_path(TICKET), ExternalReason::Closures)]
    );
}

/// A local content read of `token`, of the shape `tests/cli/local_sync.rs`
/// `an_eligible_conflict_stops_with_its_operation_and_the_way_on` reads:
/// three ticket sides that parse.
fn content(token: &str, historical: bool) -> LocalContextConflictContentDto {
    LocalContextConflictContentDto {
        operation_id: OPERATION.to_owned(),
        token: token.to_owned(),
        base: Some(ticket("Base", "open", "Body.\n")),
        local: Some(ticket("Local", "open", "Body.\n")),
        incoming: Some(ticket("Incoming", "open", "Body.\n")),
        historical,
    }
}

/// U-2. A local entry has no kind: it is classified by its display path.
/// A real local conflict on a ticket is resolvable with the inspection's
/// observation (R-11), its part the title. A document path with a
/// non-ASCII byte, escaped in its display path, does not classify, so its
/// set goes outside (K-14); no local fixture names one, so that inspection
/// is canned, of the shape `an_eligible_conflict_stops_with_its_operation_
/// and_the_way_on` reads.
#[test]
fn a_local_entry_is_classified_by_its_path() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    assert_eq!(inspection.paths.len(), 1);
    assert_eq!(inspection.paths[0].display_path, ticket_path(TICKET));

    let model = ConflictModel::from_local(&inspection, &contents);
    assert_eq!(model.observation(), inspection.observation.as_deref());
    let paths = canonical(&model);
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].display_path(), ticket_path(TICKET));
    assert_eq!(paths[0].token(), inspection.paths[0].token);
    let fields = paths[0].fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].key, "title");
    assert_eq!(fields[0].mine.as_deref(), Some("Mine"));
    assert_eq!(fields[0].theirs.as_deref(), Some("Theirs"));

    let escaped = "docs/caf\\xc3\\xa9.md";
    let model = ConflictModel::from_local(&inspection_with(escaped), &[]);
    assert_eq!(
        external(&model),
        [(escaped.to_owned(), ExternalReason::Unsupported)]
    );
}

fn inspection_with(display_path: &str) -> LocalContextSyncInspectionDto {
    let mut inspection = inspection(
        LocalContextSyncReason::CanonicalConflict,
        LocalContextSyncEligibility::EligibleCanonical,
        vec![entry(display_path, true)],
    );
    inspection.kind = "document".to_owned();
    inspection
}

#[path = "conflict_parts_tests.rs"]
mod parts;

#[path = "conflict_local_tests.rs"]
mod local;
