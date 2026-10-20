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

use super::{
    DEPENDENCY_CYCLE, INCOMPLETE, Lifecycle, OPEN_DEPENDENCY, TicketQuery, TicketRead, TicketRow,
    Tickets, UNRESOLVED_DEPENDENCY, blocked_reasons,
};
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

const BLOCKER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const WAITS: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const MISSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
const DANGLING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
const CYCLE_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
const CYCLE_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD5";
const DONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD6";
const AFTER_DONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD7";

/// A ticket waiting for an open one, one waiting for a ticket no context
/// holds, two waiting for each other, and one whose only dependency is
/// closed.
fn related(fixture: &Fixture) -> std::path::PathBuf {
    let root = fixture.create_repository("related");
    fixture.create_ticket(&root, BLOCKER, "Blocker", "open");
    fixture.create_ticket_with_deps(&root, WAITS, "Waits", "open", &[BLOCKER]);
    fixture.create_ticket_with_deps(&root, DANGLING, "Dangling", "open", &[MISSING]);
    fixture.commit_dependency_cycle(&root, (CYCLE_A, "Cycle A"), (CYCLE_B, "Cycle B"));
    fixture.commit_closed_ticket(&root, DONE, "Done");
    fixture.refresh(&root);
    fixture.create_ticket_with_deps(&root, AFTER_DONE, "After done", "open", &[DONE]);
    fixture.refresh(&root);
    root
}

fn listed(
    fixture: &Fixture,
    root: &std::path::Path,
    filter: &TicketFilter,
) -> (
    ItemListDto,
    std::collections::HashMap<String, manyhands::repository::ItemDto>,
) {
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let list = service.list_tickets(&repo, filter).unwrap();
    let all = service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let by_id = all
        .items
        .into_iter()
        .filter_map(|item| Some((item.id.clone()?, item)))
        .collect();
    (list, by_id)
}

fn item<'a>(list: &'a ItemListDto, id: &str) -> &'a manyhands::repository::ItemDto {
    list.items
        .iter()
        .find(|item| item.id.as_deref() == Some(id))
        .unwrap_or_else(|| panic!("{id} is not listed: {list:?}"))
}

/// Each reason in text: an open dependency by the blocker's short code
/// and title, an unresolved one by the ID it names, and the cycle.
#[test]
fn blocked_reasons_name_the_blocker_the_missing_id_and_the_cycle() {
    let fixture = Fixture::new();
    let root = related(&fixture);
    let (blocked, by_id) = listed(
        &fixture,
        &root,
        &TicketQuery {
            closure: ClosureFilter::Open,
            readiness: Some(ReadinessFilter::Blocked),
        }
        .to_filter(),
    );
    let blocker = by_id.get(BLOCKER).unwrap();
    let slug = blocker.slug.clone().expect("create gives a short code");

    assert_eq!(
        blocked_reasons(item(&blocked, WAITS), &by_id),
        [format!("{OPEN_DEPENDENCY}: {slug} Blocker")]
    );
    assert_eq!(
        blocked_reasons(item(&blocked, DANGLING), &by_id),
        [format!("{UNRESOLVED_DEPENDENCY}: {MISSING}")]
    );
    // A cycle reason names the tickets on it, by short code when they
    // have one (these have none) and otherwise by ID.
    let cycle = [
        format!("{OPEN_DEPENDENCY}: {CYCLE_B} Cycle B"),
        format!("{DEPENDENCY_CYCLE}: {CYCLE_A}, {CYCLE_B}"),
    ];
    assert_eq!(blocked_reasons(item(&blocked, CYCLE_A), &by_id), cycle);

    // A blocker the rows do not hold is named by its ID alone; no title
    // is made up.
    let mut without = by_id.clone();
    without.remove(BLOCKER);
    assert_eq!(
        blocked_reasons(item(&blocked, WAITS), &without),
        [format!("{OPEN_DEPENDENCY}: {BLOCKER}")]
    );
    // A ready ticket has no reasons.
    assert!(blocked_reasons(by_id.get(AFTER_DONE).unwrap(), &by_id).is_empty());
}

/// A blocker whose row has a short code but no title is named by its
/// short code, never by its ULID; nothing stands in for the title.
#[test]
fn an_untitled_blocker_is_named_by_its_short_code() {
    let fixture = Fixture::new();
    let root = related(&fixture);
    let (blocked, mut by_id) = listed(
        &fixture,
        &root,
        &TicketQuery {
            closure: ClosureFilter::Open,
            readiness: Some(ReadinessFilter::Blocked),
        }
        .to_filter(),
    );
    let blocker = by_id.get_mut(BLOCKER).unwrap();
    let slug = blocker.slug.clone().expect("create gives a short code");
    // The library's row for a ticket whose file has no title.
    blocker.title = None;
    assert_eq!(
        blocked_reasons(item(&blocked, WAITS), &by_id),
        [format!("{OPEN_DEPENDENCY}: {slug}")]
    );
}

/// A reason the library marks incomplete says so.
#[test]
fn an_incomplete_reason_is_labeled_incomplete() {
    let fixture = Fixture::new();
    let root = related(&fixture);
    let (blocked, by_id) = listed(
        &fixture,
        &root,
        &TicketFilter {
            readiness: Some(ReadinessFilter::Blocked),
            ..TicketFilter::default()
        },
    );
    let mut cycle = item(&blocked, CYCLE_A).clone();
    let readiness = cycle.readiness.as_mut().unwrap();
    for reason in &mut readiness.reasons {
        reason.complete = false;
    }
    let reasons = blocked_reasons(&cycle, &by_id);
    assert_eq!(reasons.len(), 2);
    for reason in reasons {
        assert!(reason.ends_with(&format!(" ({INCOMPLETE})")), "{reason}");
    }
}

/// Rows read under Blocked carry their reasons; under any other
/// readiness they carry none.
#[test]
fn rows_read_under_blocked_carry_their_reasons() {
    let fixture = Fixture::new();
    let root = related(&fixture);
    let (blocked, by_id) = listed(
        &fixture,
        &root,
        &TicketFilter {
            closure: ClosureFilter::Open,
            readiness: Some(ReadinessFilter::Blocked),
            ..TicketFilter::default()
        },
    );
    let names = TicketFilter::default();
    let (all, _) = listed(&fixture, &root, &names);
    let mut model = Tickets::default();
    model.load("/related".to_owned());
    let stamp = model.set_readiness(Some(ReadinessFilter::Blocked)).unwrap();
    assert_eq!(model.query().readiness, Some(ReadinessFilter::Blocked));
    assert!(model.apply(
        stamp,
        ReadResult::Ok(TicketRead {
            list: blocked.clone(),
            names: Some(all.clone()),
        })
    ));
    let rows = model.state().rows().unwrap();
    assert_eq!(rows.len(), blocked.items.len());
    for (row, item) in rows.iter().zip(&blocked.items) {
        assert_eq!(row.reasons, blocked_reasons(item, &by_id));
        assert!(!row.reasons.is_empty());
    }

    let stamp = model.set_readiness(None).unwrap();
    assert!(model.apply(stamp, ReadResult::Ok(all)));
    let rows = model.state().rows().unwrap();
    assert!(rows.iter().all(|row| row.reasons.is_empty()));
}

/// Review Focus 1: a read under the last readiness is dropped.
#[test]
fn a_readiness_change_drops_the_last_readinesss_read() {
    let fixture = Fixture::new();
    let list = empty(&fixture);
    let mut model = Tickets::default();
    let any = model.load("/a".to_owned());
    let ready = model.set_readiness(Some(ReadinessFilter::Ready)).unwrap();
    assert!(!model.apply(any, ReadResult::Ok(list.clone())));
    assert!(matches!(model.state(), ListState::Loading));
    assert!(model.apply(ready, ReadResult::Ok(list)));
    model.clear();
    assert_eq!(model.set_readiness(Some(ReadinessFilter::Blocked)), None);
    assert_eq!(model.query().readiness, Some(ReadinessFilter::Blocked));
}
