//! The edit session's pure decisions: where Edit is offered, the base a
//! draft begins on (R-11, R-72), and the toolbar's status item, which
//! reads D2's state only.
//!
//! `ItemDto` and draft fields would print text through `Debug`, so they
//! are compared with `assert!`.

use manyhands::{
    drafts::DraftFields,
    repository::{
        ClosureDto, ClosureState, DependencyDto, DependencyState, IndexState, IndexStateDto,
        ItemContextDto, ItemContextKind, ItemDto, ItemDtoKind,
    },
};
use time::{Duration, OffsetDateTime};

use super::{
    DRAFT_NOT_KEPT, DraftIndicator, DraftState, RECOVERY_OFF, base_for_edit, draft_indicator,
    edit_offered, fields_with_body, indicator_words,
};
use crate::desktop::{
    model::{editor_text::load, navigation::OpenTarget},
    test_support::fixtures::item_id,
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
const DEP_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
const DEP_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
/// CRLF line endings and a words-mixed `$$` pair: a body the editor's load
/// changes (N-1, N-2).
const RAW: &str = "First line\r\nWhat if words $$E=mc^2$$ more\r\n";

fn dependency(id: &str) -> DependencyDto {
    DependencyDto {
        id: id.to_owned(),
        state: DependencyState::Open,
    }
}

fn ticket() -> ItemDto {
    ItemDto {
        id: Some(TICKET.to_owned()),
        kind: ItemDtoKind::Ticket,
        path: format!(".manyhands/tickets/{TICKET}/ticket.md"),
        title: Some("Edited".to_owned()),
        slug: None,
        ticket_type: Some("task".to_owned()),
        status: Some("open".to_owned()),
        project: None,
        team: None,
        parent: None,
        deps: vec![dependency(DEP_B), dependency(DEP_A)],
        readiness: None,
        closure: None,
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/work/a".to_owned(),
            head_oid: Some("0123abcd".to_owned()),
        },
        observation: Some("observed".to_owned()),
        body: Some(RAW.to_owned()),
        source: Some(format!("---\nid: {TICKET}\n---\n{RAW}")),
        unknown_metadata: serde_json::Map::new(),
        changed_at: None,
        change_source: None,
        problems: Vec::new(),
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

fn tab() -> OpenTarget {
    OpenTarget::Item {
        repo: "/work/a".to_owned(),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    }
}

/// Edit is offered for a conforming item read in its own tab; not for a
/// closed ticket, a nonconforming entry, a body that could not be read, or
/// an Inspect tab, which stays the read-only source view (R-64).
#[test]
fn edit_is_offered_for_an_open_conforming_item_in_its_own_tab_only() {
    assert!(edit_offered(&tab(), &ticket()));

    let mut closed = ticket();
    closed.closure = Some(ClosureDto {
        state: ClosureState::Closed,
        closed_at: None,
        closed_by: None,
    });
    assert!(!edit_offered(&tab(), &closed), "a closed ticket");

    let mut nonconforming = ticket();
    nonconforming.id = None;
    assert!(
        !edit_offered(&tab(), &nonconforming),
        "a nonconforming entry"
    );

    let mut unread = ticket();
    unread.body = None;
    assert!(!edit_offered(&tab(), &unread), "a body that is not UTF-8");

    let inspect = OpenTarget::Inspect {
        repo: "/work/a".to_owned(),
        worktree: "/work/a".to_owned(),
        path: ticket().path,
    };
    assert!(!edit_offered(&inspect, &ticket()), "an Inspect tab");
}

/// R-11, R-72: the base holds the body as the editor loads it and the
/// `deps` in ID order; the observation, path, context and source are the
/// read's own.
#[test]
fn the_base_is_the_normalized_item() {
    let item = ticket();
    let base = base_for_edit(&item).expect("a conforming item");
    assert_eq!(base.observation.as_deref(), Some("observed"));
    assert_eq!(base.path, item.path);
    assert_eq!(base.source, item.source);
    match base.fields {
        DraftFields::Ticket { body, deps, .. } => {
            assert!(body == load(RAW), "the loaded body");
            assert!(body != RAW, "the fixture is one the load changes");
            assert_eq!(deps, [DEP_A, DEP_B], "in ID order");
        }
        _ => panic!("a ticket's fields"),
    }
    let mut nonconforming = item;
    nonconforming.id = None;
    assert!(base_for_edit(&nonconforming).is_none());
}

/// An editor's text replaces the body and nothing else.
#[test]
fn fields_with_body_replaces_the_body_only() {
    let base = base_for_edit(&ticket()).unwrap().fields;
    let edited = fields_with_body(&base, "typed".to_owned());
    match (&base, &edited) {
        (
            DraftFields::Ticket {
                title, deps, body, ..
            },
            DraftFields::Ticket {
                title: edited_title,
                deps: edited_deps,
                body: edited_body,
                ..
            },
        ) => {
            assert_eq!(title, edited_title);
            assert_eq!(deps, edited_deps);
            assert!(body != edited_body);
            assert_eq!(edited_body, "typed");
        }
        _ => panic!("a ticket's fields"),
    }
}

/// The status item reads D2's state and nothing else: the age is the last
/// write's, a write still waiting keeps the older age, nothing is said
/// before the first write, a failure is "not kept", and a store that did
/// not open is "recovery off".
#[test]
fn the_indicator_maps_each_of_d2s_states() {
    let at = OffsetDateTime::UNIX_EPOCH + Duration::days(20_000);
    assert_eq!(
        draft_indicator(DraftState::Written(at), at + Duration::seconds(7)),
        Some(DraftIndicator::Kept { age_secs: 7 })
    );
    assert_eq!(
        draft_indicator(DraftState::Written(at), at),
        Some(DraftIndicator::Kept { age_secs: 0 })
    );
    // A clock behind the write says no time has passed.
    assert_eq!(
        draft_indicator(DraftState::Written(at), at - Duration::seconds(3)),
        Some(DraftIndicator::Kept { age_secs: 0 })
    );
    assert_eq!(draft_indicator(DraftState::NotBegun, at), None);
    assert_eq!(
        draft_indicator(DraftState::Failed, at),
        Some(DraftIndicator::NotKept)
    );
    assert_eq!(
        draft_indicator(DraftState::StoreUnavailable, at),
        Some(DraftIndicator::RecoveryOff)
    );

    assert_eq!(
        indicator_words(DraftIndicator::Kept { age_secs: 12 }),
        "Draft kept \u{b7} 12s ago"
    );
    assert_eq!(indicator_words(DraftIndicator::NotKept), DRAFT_NOT_KEPT);
    assert_eq!(
        DRAFT_NOT_KEPT,
        "Draft not kept. Save to protect your changes"
    );
    assert_eq!(indicator_words(DraftIndicator::RecoveryOff), RECOVERY_OFF);
    assert_eq!(RECOVERY_OFF, "Crash recovery is off");
}
