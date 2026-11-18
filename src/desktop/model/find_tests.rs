//! The ID read behind the filter input's Enter: text that parses as a
//! ULID is an ID, and a result for an earlier read, or for a repository
//! no longer selected, is dropped.

use manyhands::{repository::ItemDto, results::ResultCode};

use super::{Find, FindResult, FindState, parse_id};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{ReadFailure, ReadResult},
    test_support::fixtures::{Fixture, item_id},
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";

/// E-5: what is an ID. Trimmed, and in either case: a ULID's letters are
/// not case-sensitive. Anything else is not an ID, and is no longer a
/// short code to look up: it filters the list.
#[test]
fn parse_id_reads_a_trimmed_ulid_in_either_case() {
    let lower = TICKET.to_ascii_lowercase();
    let short = &TICKET[1..];
    // One character short, one Crockford excludes, one too many.
    let excluded = format!("{}U", &TICKET[..25]);
    let long = format!("{TICKET}0");
    let spaced = format!("  {lower}\n");
    let tabbed = format!("\t{TICKET} ");
    let table: [(&str, Option<&str>); 10] = [
        (TICKET, Some(TICKET)),
        (&lower, Some(TICKET)),
        (&spaced, Some(TICKET)),
        (&tabbed, Some(TICKET)),
        (" ae-k9x2b ", None),
        (short, None),
        (&excluded, None),
        (&long, None),
        ("", None),
        (" \t ", None),
    ];
    for (text, id) in table {
        assert_eq!(parse_id(text), id.map(item_id), "{text:?}");
    }
}

fn found() -> (ItemDto, String) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Found", "open");
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let item = service.show_item(&repo, &item_id(TICKET)).unwrap();
    (item, root.to_str().unwrap().to_owned())
}

#[test]
fn an_id_found_opens_by_its_id_and_one_not_found_is_no_match() {
    let (item, a) = found();

    let mut find = Find::default();
    let stamp = find.start(a.clone());
    assert_eq!(find.state(), &FindState::Searching);
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

    // Busy is its own state, and any other failure is shown by its code.
    let stamp = find.start(a.clone());
    assert_eq!(find.apply_item(stamp, ReadResult::Busy), None);
    assert_eq!(find.state(), &FindState::Busy);
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

/// Review Focus 4: a read lands only while it is the last one made, and
/// not after Find was cleared for another repository.
#[test]
fn an_earlier_search_is_dropped() {
    let (item, a) = found();
    let mut find = Find::default();
    let first = find.start(a.clone());
    let second = find.start(a.clone());
    assert_eq!(
        find.apply_item(first, ReadResult::Ok(item.clone())),
        None,
        "an earlier read opens nothing"
    );
    assert_eq!(find.state(), &FindState::Searching);
    assert!(
        find.apply_item(second, ReadResult::Ok(item.clone()))
            .is_some()
    );

    let third = find.start(a);
    find.clear();
    assert_eq!(find.state(), &FindState::Idle);
    assert_eq!(find.apply_item(third, ReadResult::Ok(item)), None);
    assert_eq!(find.state(), &FindState::Idle);
}
