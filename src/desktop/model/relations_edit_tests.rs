//! The relationship decisions, without a repository: the words of each
//! rejection under each field, Save's reasons, where a save's answer goes,
//! the ID order of a dependency added and removed (R-72), an answer from
//! an index that is behind, and what the picker offers.

use manyhands::{
    drafts::DraftFields,
    repository::{IndexState, IndexStateDto, RelationshipCheckDto, RelationshipRejectionDto},
    results::ResultCode,
};

use super::*;

const OWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG2";
const C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG3";
const D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG4";

fn rejection(code: ResultCode, ids: &[&str]) -> Rejection {
    Rejection {
        code,
        ids: ids.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// A short name for each ID, as a loaded row would give it.
fn named(id: &str) -> String {
    match id {
        OWN => "MH-1 Own".to_owned(),
        B => "MH-2 Bee".to_owned(),
        other => other.to_owned(),
    }
}

fn proposed(deps: &[&str], parent: Option<&str>) -> Proposed {
    Proposed {
        deps: deps.iter().map(|id| (*id).to_owned()).collect(),
        parent: parent.map(str::to_owned),
    }
}

fn check(
    rejected: Option<(ResultCode, &[&str])>,
    complete: bool,
    state: IndexState,
) -> RelationshipCheckDto {
    RelationshipCheckDto {
        deps: Vec::new(),
        parent: None,
        unresolved: vec![D.to_owned()],
        rejection: rejected.map(|(code, ids)| RelationshipRejectionDto {
            code,
            ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        }),
        complete,
        index: IndexStateDto {
            state,
            refreshed_at: None,
        },
    }
}

fn candidate(id: &str, slug: &str, title: &str) -> Candidate {
    Candidate {
        id: id.to_owned(),
        slug: Some(slug.to_owned()),
        title: Some(title.to_owned()),
        lifecycle: Some(Lifecycle::Open),
    }
}

/// Each rejection in the ruling's words, its tickets named and joined
/// with ", ", or by ID where no row names them; the ticket alone is the
/// ticket naming itself, in its field's words.
#[test]
fn each_rejection_is_explained_in_its_fields_words() {
    for field in [RelationField::WaitingOn, RelationField::Parent] {
        let cycle = rejection(ResultCode::RelationshipCycle, &[OWN, B, C]);
        assert_eq!(
            explanation(&cycle, field, OWN, named).as_deref(),
            Some(format!("This makes a cycle: MH-1 Own, MH-2 Bee, {C}.").as_str()),
        );
        let invalid = rejection(ResultCode::InvalidRelationship, &[C]);
        assert_eq!(
            explanation(&invalid, field, OWN, named).as_deref(),
            Some(format!("Not a ticket: {C}.").as_str()),
        );
    }
    let alone = rejection(ResultCode::RelationshipCycle, &[OWN]);
    assert_eq!(
        explanation(&alone, RelationField::WaitingOn, OWN, named).as_deref(),
        Some("A ticket cannot depend on itself.")
    );
    assert_eq!(
        explanation(&alone, RelationField::Parent, OWN, named).as_deref(),
        Some("A ticket cannot be its own parent.")
    );
    // Another ticket alone is no self-reference.
    let other = rejection(ResultCode::RelationshipCycle, &[B]);
    assert_eq!(
        explanation(&other, RelationField::Parent, OWN, named).as_deref(),
        Some("This makes a cycle: MH-2 Bee.")
    );
    let unknown = rejection(ResultCode::Busy, &[B]);
    assert_eq!(
        explanation(&unknown, RelationField::Parent, OWN, named),
        None
    );
}

/// Save's two reasons, and none for any other code; a save's answer is
/// under Waiting on when the dependencies it sent differ from the base's.
#[test]
fn save_has_a_reason_for_each_rejection_and_its_answer_a_field() {
    assert_eq!(
        save_reason(ResultCode::RelationshipCycle),
        Some("A ticket cannot be saved with a relationship cycle.")
    );
    assert_eq!(
        save_reason(ResultCode::InvalidRelationship),
        Some("A ticket's dependencies and parent must be tickets.")
    );
    assert_eq!(save_reason(ResultCode::ExternalChange), None);

    let base = proposed(&[B], Some(C));
    assert_eq!(
        save_field(&proposed(&[B, D], Some(C)), &base),
        RelationField::WaitingOn
    );
    assert_eq!(
        save_field(&proposed(&[B], Some(D)), &base),
        RelationField::Parent
    );
    assert_eq!(
        save_field(&proposed(&[], None), &base),
        RelationField::WaitingOn
    );
    let answered = Answer::of_save(
        proposed(&[B], Some(D)),
        &base,
        ResultCode::RelationshipCycle,
        vec![OWN.to_owned(), D.to_owned()],
        None,
    )
    .expect("a rejection's code");
    assert_eq!(answered.field, RelationField::Parent);
    assert!(answered.rejection == Some(rejection(ResultCode::RelationshipCycle, &[OWN, D])));
    assert!(
        Answer::of_save(
            proposed(&[B], None),
            &base,
            ResultCode::ExternalChange,
            Vec::new(),
            None
        )
        .is_none()
    );
}

/// Fix 1: a save's refusal changes only the rejection. What the check said
/// of the same proposal's unresolved IDs and index is kept, and nothing of
/// another proposal's; a check of the same proposal landing after it keeps
/// the refusal's rejection and field, and gives its own unresolved IDs.
#[test]
fn a_saves_refusal_keeps_what_the_check_said_and_a_later_check_keeps_the_refusal() {
    let base = proposed(&[], None);
    let mine = proposed(&[B, D], None);
    let checked = Answer::of_check(mine.clone(), RelationField::WaitingOn, {
        let mut answer = check(None, true, IndexState::Stale);
        answer.unresolved = vec![D.to_owned()];
        answer
    });
    let refused = Answer::of_save(
        mine.clone(),
        &base,
        ResultCode::RelationshipCycle,
        vec![OWN.to_owned(), B.to_owned()],
        Some(&checked),
    )
    .expect("a rejection's code");
    assert!(refused.unresolved == [D], "the check's unresolved IDs stay");
    assert!(refused.partial, "and what it said of the index");
    assert!(refused.refused);
    assert!(refused.rejection == Some(rejection(ResultCode::RelationshipCycle, &[OWN, B])));

    // Another proposal's check says nothing of this one.
    let other = Answer::of_check(proposed(&[B], None), RelationField::WaitingOn, {
        let mut answer = check(None, false, IndexState::Current);
        answer.unresolved = vec![D.to_owned()];
        answer
    });
    let alone = Answer::of_save(
        mine.clone(),
        &base,
        ResultCode::RelationshipCycle,
        vec![OWN.to_owned(), B.to_owned()],
        Some(&other),
    )
    .expect("a rejection's code");
    assert!(alone.unresolved.is_empty());
    assert!(!alone.partial);

    // A check of the same proposal, landing after the refusal, finds no
    // cycle: the refusal's rejection and field stay, its IDs are taken.
    let later = Answer::of_check(mine.clone(), RelationField::Parent, {
        let mut answer = check(None, true, IndexState::Current);
        answer.unresolved = vec![D.to_owned()];
        answer
    })
    .after(&alone);
    assert!(later.rejection == alone.rejection);
    assert_eq!(later.field, RelationField::WaitingOn);
    assert!(later.refused);
    assert!(later.unresolved == [D]);
    assert!(!later.partial);
    // After a check's answer, or one of another proposal, nothing is kept.
    let plain = Answer::of_check(
        mine,
        RelationField::Parent,
        check(None, true, IndexState::Current),
    )
    .after(&checked);
    assert!(plain.rejection.is_none());
    assert!(!plain.refused);
}

/// R-72: a dependency goes in where its ID sorts, once; removing it leaves
/// the proposal as it was, and its fields equal to the base's; a parent is
/// set and cleared.
#[test]
fn a_dependency_goes_in_id_order_and_out_again_to_the_base() {
    let base = proposed(&[B, D], None);
    let added = base.with_dependency(C);
    assert!(added.deps == [B, C, D]);
    assert!(added.with_dependency(C) == added, "once");
    assert!(base.with_dependency(OWN).deps == [OWN, B, D]);
    let removed = added.without_dependency_at(1);
    assert!(removed == base, "back to the base exactly");
    assert!(removed.without_dependency_at(9) == base);

    let fields = DraftFields::Ticket {
        title: "T".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: base.deps.clone(),
        parent: None,
        body: String::new(),
    };
    let changed = added.with_parent(Some(B)).applied_to(&fields);
    assert!(Proposed::of(&changed) == Some(proposed(&[B, C, D], Some(B))));
    let back = base.applied_to(&changed);
    assert!(back == fields, "the base's fields again");
    assert!(
        added
            .with_parent(Some(B))
            .with_parent(None)
            .parent
            .is_none()
    );
    assert!(
        Proposed::of(&DraftFields::Comment {
            body: String::new()
        })
        .is_none()
    );
    assert!(proposed(&["not-an-id"], None).request().is_none());
    let request = proposed(&[B], Some(C)).request().expect("IDs");
    assert_eq!(request.deps.len(), 1);
    assert!(request.parent.is_some());
}

/// An answer is partial, and says Save checks again, when its index is not
/// current or it is incomplete; it keeps the check's rejection and
/// unresolved IDs.
#[test]
fn an_answer_from_an_index_behind_or_incomplete_is_partial() {
    let answer = |check| Answer::of_check(proposed(&[B], None), RelationField::WaitingOn, check);
    let current = answer(check(None, true, IndexState::Current));
    assert!(!current.partial);
    assert!(current.rejection.is_none());
    assert!(current.unresolved == [D]);
    assert!(answer(check(None, false, IndexState::Current)).partial);
    assert!(answer(check(None, true, IndexState::Stale)).partial);
    assert!(answer(check(None, true, IndexState::NeverRefreshed)).partial);
    let rejected = answer(check(
        Some((ResultCode::RelationshipCycle, &[OWN, B])),
        true,
        IndexState::Current,
    ));
    assert!(rejected.rejection == Some(rejection(ResultCode::RelationshipCycle, &[OWN, B])));
}

/// The picker: exact short-code matches first, then the titles that hold
/// the filter, ignoring case; every listed ticket while it is empty; never
/// one left out; each once; at most eight.
#[test]
fn the_picker_offers_exact_short_codes_then_matching_titles() {
    let listed = vec![
        candidate(OWN, "mh-own", "Own"),
        candidate(B, "mh-login", "Login page"),
        candidate(C, "mh-logout", "Logout page"),
        candidate(D, "mh-search", "Search box"),
    ];
    let ids = |offered: Vec<Candidate>| -> Vec<String> {
        offered.into_iter().map(|candidate| candidate.id).collect()
    };
    assert_eq!(ids(offered(&listed, &[], "", &[OWN])), [B, C, D]);
    assert_eq!(ids(offered(&listed, &[], "LOG", &[OWN])), [B, C]);
    assert_eq!(
        ids(offered(&listed, &[], "login", &[OWN, B])),
        Vec::<String>::new()
    );
    // A short code is matched only exactly, through the library.
    assert_eq!(
        ids(offered(&listed, &[], "mh-search", &[OWN])),
        Vec::<String>::new()
    );
    let exact = vec![candidate(D, "mh-search", "Search box")];
    assert_eq!(ids(offered(&listed, &exact, "mh-search", &[OWN])), [D]);
    let exact = vec![candidate(C, "page", "Logout page")];
    assert_eq!(
        ids(offered(&listed, &exact, "page", &[OWN])),
        [C, B],
        "once"
    );
    let many: Vec<Candidate> = (0..12)
        .map(|n| candidate(&format!("01ARZ3NDEKTSV4RRFFQ69G5F{n:02}"), "s", "Many"))
        .collect();
    assert_eq!(offered(&many, &[], "many", &[]).len(), 8);
    assert_eq!(
        candidate(B, "mh-login", "Login page").named(),
        "mh-login Login page"
    );
    let unnamed = Candidate {
        id: B.to_owned(),
        slug: None,
        title: None,
        lifecycle: None,
    };
    assert_eq!(unnamed.named(), B);
}
