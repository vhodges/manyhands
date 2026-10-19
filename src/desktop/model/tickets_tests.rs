//! The ticket query, rows made from the library's entries, and the
//! model's stamps: a filter change drops the read made under the last
//! filter.

use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureFilter, ClosureState, ItemDtoKind, ItemListDto, ReadinessFilter, TicketFilter,
    },
    results::ProblemCode,
};

use super::{Lifecycle, TicketQuery, TicketRow, Tickets};
use crate::desktop::{
    model::{list_state::ListState, navigation::OpenTarget},
    services::ReadResult,
    test_support::fixtures::Fixture,
};

#[test]
fn the_default_query_lists_open_tickets() {
    assert_eq!(
        TicketQuery::default().to_filter().closure,
        ClosureFilter::Open
    );
    assert_eq!(
        TicketQuery::default().to_filter(),
        TicketFilter {
            closure: ClosureFilter::Open,
            ..TicketFilter::default()
        }
    );
}

#[test]
fn the_query_maps_closure_and_readiness_and_nothing_else() {
    let query = TicketQuery {
        closure: ClosureFilter::All,
        readiness: Some(ReadinessFilter::Blocked),
    };
    assert_eq!(
        query.to_filter(),
        TicketFilter {
            closure: ClosureFilter::All,
            readiness: Some(ReadinessFilter::Blocked),
            ..TicketFilter::default()
        }
    );
}

const OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";

/// Rows are the library's entries: status as written, lifecycle from
/// closure, and a nonconforming entry by its path and problem.
#[test]
fn rows_are_made_from_the_librarys_entries() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, OPEN, "Says closed", "closed");
    fixture.commit_closed_ticket(&root, CLOSED, "Closed");
    fixture.commit_nonconforming_ticket(&root, BROKEN);
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let list = service
        .list_tickets(
            &repo,
            &TicketFilter {
                closure: ClosureFilter::All,
                ..TicketFilter::default()
            },
        )
        .unwrap();

    let rows: Vec<TicketRow> = list.items.iter().map(TicketRow::from_item).collect();
    assert_eq!(rows.len(), 3, "{list:?}");
    for (row, item) in rows.iter().zip(&list.items) {
        assert_eq!(row.slug, item.slug);
        assert_eq!(row.title, item.title);
        assert_eq!(row.ticket_type, item.ticket_type);
        assert_eq!(row.status, item.status);
        assert_eq!(row.changed_at, item.changed_at);
        assert_eq!(row.path, item.path);
        let lifecycle = item.closure.as_ref().map(|closure| match closure.state {
            ClosureState::Open => Lifecycle::Open,
            ClosureState::Closed => Lifecycle::Closed,
        });
        assert_eq!(row.lifecycle, lifecycle);
    }
    let by_id = |id: &str| {
        rows.iter()
            .find(|row| row.id.as_ref().map(ItemId::to_string).as_deref() == Some(id))
            .unwrap()
    };
    let says_closed = by_id(OPEN);
    assert_eq!(says_closed.status.as_deref(), Some("closed"));
    assert_eq!(says_closed.lifecycle, Some(Lifecycle::Open));
    assert_eq!(says_closed.nonconforming, None);
    assert_eq!(by_id(CLOSED).lifecycle, Some(Lifecycle::Closed));

    let broken = rows.iter().find(|row| row.id.is_none()).unwrap();
    assert_eq!(
        broken.nonconforming,
        Some(ProblemCode::MalformedFrontMatter)
    );
    assert_eq!(broken.lifecycle, None);
    assert_eq!(broken.title, None);

    // A row opens by ID, or by path when it has none.
    let key = "/a".to_owned();
    assert_eq!(
        says_closed.target(&key),
        OpenTarget::Item {
            repo: key.clone(),
            kind: ItemDtoKind::Ticket,
            id: OPEN.parse().unwrap(),
        }
    );
    assert_eq!(
        broken.target(&key),
        OpenTarget::Inspect {
            repo: key.clone(),
            worktree: list.items[2].context.worktree.clone(),
            path: broken.path.clone(),
        }
    );
}

fn empty(fixture: &Fixture) -> ItemListDto {
    let root = fixture.create_repository("empty");
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap()
}

/// Review Focus 1: a read under the last filter, or for the last
/// repository, is dropped when it completes.
#[test]
fn a_filter_change_drops_the_last_filters_read() {
    let fixture = Fixture::new();
    let list = empty(&fixture);
    let mut model = Tickets::default();
    let open = model.load("/a".to_owned());
    let closed = model.set_closure(ClosureFilter::Closed).unwrap();
    assert_eq!(model.query().closure, ClosureFilter::Closed);
    assert!(!model.apply(open, ReadResult::Ok(list.clone())));
    assert!(matches!(model.state(), ListState::Loading));
    assert!(model.apply(closed, ReadResult::Ok(list.clone())));
    assert!(matches!(model.state(), ListState::Ready { .. }));

    let a = model.load("/a".to_owned());
    let _b = model.load("/b".to_owned());
    assert!(!model.apply(a, ReadResult::Ok(list.clone())));

    let b = model.load("/b".to_owned());
    model.clear();
    assert!(!model.apply(b, ReadResult::Ok(list)));
    assert_eq!(model.repo(), None);
    // With nothing loaded, a filter change reads nothing, and is kept.
    assert_eq!(model.set_closure(ClosureFilter::All), None);
    assert_eq!(model.query().closure, ClosureFilter::All);
}
