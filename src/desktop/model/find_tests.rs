//! Find's query and its stamps: text that parses as a ULID is an ID,
//! anything else not blank is a short code, and a result for an earlier
//! search, or for a repository no longer searched, is dropped.

use manyhands::{repository::ItemListDto, results::ResultCode};

use super::{Find, FindQuery, FindResult, FindState};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{ReadFailure, ReadResult},
    test_support::fixtures::{Fixture, item_id},
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";

#[test]
fn a_ulid_is_an_id_and_anything_else_is_a_short_code() {
    assert_eq!(
        FindQuery::parse(TICKET),
        Some(FindQuery::Id(item_id(TICKET)))
    );
    // Trimmed, and in either case: a ULID's letters are not case-sensitive.
    assert_eq!(
        FindQuery::parse(&format!("  {}\n", TICKET.to_ascii_lowercase())),
        Some(FindQuery::Id(item_id(TICKET)))
    );
    assert_eq!(
        FindQuery::parse(" ae-k9x2b "),
        Some(FindQuery::Slug("ae-k9x2b".to_owned()))
    );
    // Not quite a ULID: one character short, or one Crockford excludes.
    assert_eq!(
        FindQuery::parse(&TICKET[1..]),
        Some(FindQuery::Slug(TICKET[1..].to_owned()))
    );
    let excluded = format!("{}U", &TICKET[..25]);
    assert_eq!(
        FindQuery::parse(&excluded),
        Some(FindQuery::Slug(excluded.clone()))
    );
    assert_eq!(FindQuery::parse(""), None);
    assert_eq!(FindQuery::parse(" \t "), None);
}

fn found(fixture: &Fixture, slug: &str) -> ItemListDto {
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Found", "open");
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    service.find_tickets_by_slug(&repo, slug).unwrap()
}

#[test]
fn matches_are_the_librarys_and_none_says_so() {
    let fixture = Fixture::new();
    let none = found(&fixture, "zz-zzzzz");
    let mut find = Find::default();
    let stamp = find.start("/a".to_owned());
    assert!(matches!(find.state(), FindState::Searching));
    assert!(find.apply_matches(stamp, ReadResult::Ok(none)));
    assert_eq!(find.state(), &FindState::Done(FindResult::None));
}

#[test]
fn an_id_found_opens_by_its_id_and_one_not_found_is_no_match() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Found", "open");
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let item = service.show_item(&repo, &item_id(TICKET)).unwrap();
    let a = root.to_str().unwrap().to_owned();

    let mut find = Find::default();
    let stamp = find.start(a.clone());
    assert_eq!(
        find.apply_item(stamp, ReadResult::Ok(item.clone())),
        Some(OpenTarget::for_item(&a, &item))
    );
    assert_eq!(find.state(), &FindState::Done(FindResult::Opened));

    let stamp = find.start(a.clone());
    let missing = ReadResult::Failed(ReadFailure {
        code: ResultCode::ItemNotFound,
        recovery: Vec::new(),
    });
    assert_eq!(find.apply_item(stamp, missing), None);
    assert_eq!(find.state(), &FindState::Done(FindResult::None));

    // Any other failure is shown by its code.
    let stamp = find.start(a);
    let failure = ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: Vec::new(),
    };
    assert_eq!(
        find.apply_item(stamp, ReadResult::Failed(failure.clone())),
        None
    );
    assert_eq!(find.state(), &FindState::Failed(failure));
}

/// Review Focus 1: a search's result lands only while it is the last one
/// made, and not after Find was cleared for another repository.
#[test]
fn an_earlier_search_is_dropped() {
    let fixture = Fixture::new();
    let none = found(&fixture, "zz-zzzzz");
    let mut find = Find::default();
    let first = find.start("/a".to_owned());
    let second = find.start("/a".to_owned());
    assert!(!find.apply_matches(first, ReadResult::Ok(none.clone())));
    assert!(matches!(find.state(), FindState::Searching));
    assert!(find.apply_matches(second, ReadResult::Ok(none.clone())));

    let third = find.start("/a".to_owned());
    find.clear();
    assert_eq!(find.state(), &FindState::Idle);
    assert!(!find.apply_matches(third, ReadResult::Ok(none)));
    assert_eq!(find.state(), &FindState::Idle);
}
