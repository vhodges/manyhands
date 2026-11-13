//! The ticket query, rows made from the library's entries, and the
//! model's stamps: a read lands only for the repository and load it was
//! made for.

use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureFilter, ClosureState, ItemDtoKind, ItemListDto, ReadinessFilter, ReadinessState,
        TicketFilter,
    },
    results::{ProblemCode, ResultCode},
};

use super::{
    DEPENDENCY_CYCLE, INCOMPLETE, Lifecycle, OPEN_DEPENDENCY, SortColumn, TicketQuery, TicketRow,
    TicketSort, Tickets, UNRESOLVED_DEPENDENCY, blocked_reasons, read_filter,
};
use crate::desktop::{
    model::{
        list_state::ListState, navigation::OpenTarget, stamp::Stamp, unreadable::UnreadableFile,
    },
    services::{ReadFailure, ReadResult},
    test_support::fixtures::Fixture,
};

/// The default query lists open tickets, ready or not, unfiltered, in the
/// default order; the one read is the library's default filter whatever
/// the query (R-1).
#[test]
fn the_default_query_lists_open_tickets() {
    let query = TicketQuery::default();
    assert_eq!(query.closure, ClosureFilter::Open);
    assert_eq!(query.readiness, None);
    assert_eq!(query.text, "");
    assert_eq!(
        query.sort,
        TicketSort {
            column: SortColumn::Updated,
            ascending: true,
        }
    );
    assert_eq!(read_filter(), TicketFilter::default());
}

/// Text or a readiness narrows the list; a lifecycle or the sort does not.
#[test]
fn only_text_or_readiness_narrows_the_query() {
    for (name, query, narrowed) in [
        ("the default", TicketQuery::default(), false),
        ("blank text", text(" \t"), false),
        ("text", text(" dock "), true),
        (
            "Ready",
            TicketQuery::default().with_readiness(Some(ReadinessFilter::Ready)),
            true,
        ),
        (
            "Closed and sorted",
            TicketQuery {
                sort: TicketSort::default().toggled(SortColumn::Title),
                ..TicketQuery::default().with_closure(ClosureFilter::Closed)
            },
            false,
        ),
    ] {
        assert_eq!(query.narrowed(), narrowed, "{name}");
    }
}

fn text(text: &str) -> TicketQuery {
    TicketQuery {
        text: text.to_owned(),
        ..TicketQuery::default()
    }
}

/// A new column sorts ascending; the same column again reverses.
#[test]
fn a_sort_toggles_by_column() {
    let (code, title) = (SortColumn::Code, SortColumn::Title);
    let sort = |column, ascending| TicketSort { column, ascending };
    for (from, column, to) in [
        (
            sort(SortColumn::Updated, true),
            SortColumn::Updated,
            sort(SortColumn::Updated, false),
        ),
        (
            sort(SortColumn::Updated, false),
            SortColumn::Updated,
            sort(SortColumn::Updated, true),
        ),
        (sort(SortColumn::Updated, true), code, sort(code, true)),
        (sort(code, false), title, sort(title, true)),
        (sort(title, true), title, sort(title, false)),
    ] {
        assert_eq!(from.toggled(column), to, "{from:?} then {column:?}");
    }
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

/// Review Focus 4: a read lands only for the repository and the load it
/// was made for, and its blocked rows carry reasons named from all its
/// rows. A reload of the same repository keeps the rows; another
/// repository drops them. A query change reads nothing and derives at
/// once, and a clear does not forget the last-loaded repository.
#[test]
fn a_read_lands_only_for_its_repository_and_load() {
    struct Run {
        model: Tickets,
        stamps: Vec<Stamp>,
        list: ItemListDto,
        by_id: std::collections::HashMap<String, manyhands::repository::ItemDto>,
    }
    type Step = (&'static str, Box<dyn Fn(&mut Run)>);

    let fixture = Fixture::new();
    let root = related(&fixture);
    let (list, by_id) = listed(&fixture, &root, &TicketFilter::default());
    let blocked_items = list
        .items
        .iter()
        .filter(|item| {
            item.readiness
                .as_ref()
                .is_some_and(|readiness| readiness.state == ReadinessState::Blocked)
        })
        .count();
    let open_items = list
        .items
        .iter()
        .filter(|item| {
            item.closure
                .as_ref()
                .is_some_and(|closure| closure.state == ClosureState::Open)
        })
        .count();
    assert!(blocked_items > 0 && blocked_items < open_items, "{list:?}");
    let blocked = TicketQuery {
        closure: ClosureFilter::All,
        readiness: Some(ReadinessFilter::Blocked),
        ..TicketQuery::default()
    };

    fn load(run: &mut Run, repo: &str, changed: bool) {
        let (stamp, was) = run.model.load(repo.to_owned());
        assert_eq!(was, changed, "{repo} changed the repository");
        run.stamps.push(stamp);
    }
    fn apply(run: &mut Run, stamp: usize, landed: bool) {
        let read = ReadResult::Ok(run.list.clone());
        assert_eq!(run.model.apply(run.stamps[stamp].clone(), read), landed);
    }
    fn file(name: &str) -> UnreadableFile {
        UnreadableFile {
            path: format!(".manyhands/tickets/{name}/ticket.md"),
            worktree: "/a".to_owned(),
            primary: true,
        }
    }
    /// The status read of load `stamp` answers `result`.
    fn status(run: &mut Run, stamp: usize, result: ReadResult<Vec<UnreadableFile>>, landed: bool) {
        assert_eq!(
            run.model
                .apply_unreadable(run.stamps[stamp].clone(), result),
            landed
        );
    }
    fn failed() -> ReadResult<Vec<UnreadableFile>> {
        ReadResult::Failed(ReadFailure {
            code: ResultCode::RepositoryInaccessible,
            recovery: Vec::new(),
        })
    }
    /// The files the model holds, and that its view lists them last, after
    /// every other row, counted apart from the tickets shown (R-9, P6).
    fn holds(run: &Run, names: &[&str]) {
        let files: Vec<_> = names.iter().map(|name| file(name)).collect();
        assert_eq!(run.model.unreadable(), files);
        let view = run.model.view();
        assert_eq!(view.unreadable, files.len());
        let tail: Vec<_> = view
            .rows
            .iter()
            .filter_map(|row| row.unreadable.clone())
            .collect();
        assert_eq!(tail, files, "listed in the status's order");
        assert!(
            view.rows
                .iter()
                .skip(view.rows.len() - files.len())
                .all(|row| row.unreadable.is_some()),
            "after every other row"
        );
    }

    let steps: Vec<Step> = vec![
        (
            "a query set before any load is kept and derives",
            Box::new({
                let blocked = blocked.clone();
                move |run| {
                    run.model.set_query(blocked.clone());
                    assert_eq!(run.model.query(), &blocked);
                    assert_eq!(run.model.repo(), None);
                    assert_eq!(run.model.view().listed, 0);
                }
            }),
        ),
        (
            "a first load is a change of repository",
            Box::new(|run| load(run, "/a", true)),
        ),
        (
            "the read for the load in flight lands, blocked rows with reasons from all rows",
            Box::new(move |run| {
                apply(run, 0, true);
                let rows = run.model.list().rows().unwrap();
                assert_eq!(rows.len(), run.list.items.len());
                for (row, item) in rows.iter().zip(&run.list.items) {
                    let reasons = if row.readiness == Some(ReadinessState::Blocked) {
                        blocked_reasons(item, &run.by_id)
                    } else {
                        Vec::new()
                    };
                    assert_eq!(row.reasons, reasons, "{:?}", row.title);
                    assert_eq!(
                        row.readiness,
                        item.readiness.as_ref().map(|readiness| readiness.state)
                    );
                    assert_eq!(row.context.worktree, item.context.worktree);
                    assert_eq!(row.context.kind, item.context.kind);
                    assert_eq!(row.context.branch, item.context.branch);
                }
                assert_eq!(
                    rows.iter().filter(|row| !row.reasons.is_empty()).count(),
                    blocked_items
                );
                // The query kept before the load shows its blocked rows.
                assert_eq!(run.model.view().listed, blocked_items);
                assert!(
                    run.model
                        .view()
                        .rows
                        .iter()
                        .all(|row| row.readiness == Some(ReadinessState::Blocked))
                );
            }),
        ),
        (
            "a query change starts no read, derives at once and keeps the read in flight wanted",
            Box::new(move |run| {
                run.model.set_query(TicketQuery::default());
                assert_eq!(run.model.view().listed, open_items);
                assert!(matches!(run.model.state(), ListState::Ready { .. }));
                apply(run, 0, true);
                // Reasons are filled for every blocked row whatever the
                // query, so the Open view carries them too (R-18).
                let kept = run.model.list().rows().unwrap();
                for (row, item) in kept.iter().zip(&run.list.items) {
                    if row.readiness == Some(ReadinessState::Blocked) {
                        assert_eq!(row.reasons, blocked_reasons(item, &run.by_id));
                    }
                }
                let blocked_in_view: Vec<_> = run
                    .model
                    .view()
                    .rows
                    .iter()
                    .filter(|row| row.readiness == Some(ReadinessState::Blocked))
                    .collect();
                assert_eq!(blocked_in_view.len(), blocked_items);
                assert!(blocked_in_view.iter().all(|row| !row.reasons.is_empty()));
            }),
        ),
        (
            "a status read that lands replaces the unreadable files, listed last",
            Box::new(|run| {
                holds(run, &[]);
                status(run, 0, ReadResult::Ok(vec![file("U1")]), true);
                holds(run, &["U1"]);
                status(run, 0, ReadResult::Ok(vec![file("U1"), file("U2")]), true);
                holds(run, &["U1", "U2"]);
            }),
        ),
        (
            "a busy or a failed status read keeps them",
            Box::new(|run| {
                status(run, 0, ReadResult::Busy, true);
                holds(run, &["U1", "U2"]);
                status(run, 0, failed(), true);
                holds(run, &["U1", "U2"]);
            }),
        ),
        (
            "a reload of the same repository keeps its rows and is no change",
            Box::new(|run| {
                load(run, "/a", false);
                assert!(run.model.list().reloading());
                assert!(!run.model.view().rows.is_empty());
                // The unreadable files stay while the reload runs.
                holds(run, &["U1", "U2"]);
            }),
        ),
        (
            "a read for an earlier load is dropped, a status read too",
            Box::new(|run| {
                apply(run, 0, false);
                assert!(run.model.list().reloading());
                status(run, 0, ReadResult::Ok(Vec::new()), false);
                holds(run, &["U1", "U2"]);
            }),
        ),
        (
            "the read of the current load lands, and its status read replaces the files",
            Box::new(|run| {
                apply(run, 1, true);
                assert!(!run.model.list().reloading());
                holds(run, &["U1", "U2"]);
                status(run, 1, ReadResult::Ok(vec![file("U3")]), true);
                holds(run, &["U3"]);
                status(run, 1, ReadResult::Ok(Vec::new()), true);
                holds(run, &[]);
                status(run, 1, ReadResult::Ok(vec![file("U3")]), true);
                holds(run, &["U3"]);
            }),
        ),
        (
            "another repository is a change and drops the kept rows and the files",
            Box::new(|run| {
                load(run, "/b", true);
                assert!(run.model.list().rows().is_none());
                assert_eq!(run.model.view().listed, 0);
                assert_eq!(run.model.view().counts, None);
                assert!(run.model.unreadable().is_empty());
            }),
        ),
        (
            "a read for the repository left is dropped, a status read too",
            Box::new(|run| {
                apply(run, 1, false);
                status(run, 1, ReadResult::Ok(vec![file("U4")]), false);
                assert!(run.model.unreadable().is_empty());
                // A status read may land before the tickets: nothing shows
                // until they do.
                status(run, 2, ReadResult::Ok(vec![file("U5")]), true);
                assert_eq!(run.model.view().listed, 0);
                apply(run, 2, true);
                holds(run, &["U5"]);
            }),
        ),
        (
            "a read after clear is dropped",
            Box::new(|run| {
                run.model.clear();
                apply(run, 2, false);
                status(run, 2, ReadResult::Ok(vec![file("U6")]), false);
                assert!(run.model.unreadable().is_empty());
                assert_eq!(run.model.repo(), None);
                assert!(matches!(run.model.state(), ListState::Loading));
                assert!(run.model.list().rows().is_none());
            }),
        ),
        (
            "after clear, the last-loaded repository is no change",
            Box::new(|run| load(run, "/b", false)),
        ),
        (
            "and another repository is",
            Box::new(|run| {
                run.model.clear();
                load(run, "/c", true);
            }),
        ),
    ];

    let mut run = Run {
        model: Tickets::default(),
        stamps: Vec::new(),
        list,
        by_id,
    };
    for (name, step) in steps {
        eprintln!("{name}");
        step(&mut run);
    }
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
        &TicketFilter {
            closure: ClosureFilter::Open,
            readiness: Some(ReadinessFilter::Blocked),
            ..TicketFilter::default()
        },
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
        &TicketFilter {
            closure: ClosureFilter::Open,
            readiness: Some(ReadinessFilter::Blocked),
            ..TicketFilter::default()
        },
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
