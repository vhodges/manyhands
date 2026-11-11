//! Fixtures for `ticket_view_tests`: the library's six entries and the
//! queries that sort them.

use manyhands::{
    repository::{
        ClosureDto, ClosureFilter, ClosureState, IndexState, IndexStateDto, ItemContextDto,
        ItemContextKind, ItemDto, ItemDtoKind, ProblemDto, ReadinessDto, ReadinessFilter,
        ReadinessReasonCode, ReadinessReasonDto, ReadinessState,
    },
    results::ProblemCode,
};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::desktop::model::{
    markers::RowContext,
    tickets::{SortColumn, TicketQuery, TicketRow, TicketSort},
};

pub(super) const REPO: &str = "manyhands";
pub(super) const ID_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";
pub(super) const ID_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
pub(super) const ID_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
pub(super) const ID_D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
pub(super) const ID_E: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";

pub(super) fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-11-11T12:00:00Z", &Rfc3339).unwrap()
}

pub(super) fn ago(minutes: i64) -> String {
    (now() - Duration::minutes(minutes))
        .format(&Rfc3339)
        .unwrap()
}

/// What one fixture ticket is, written out as the library lists it.
pub(super) struct Spec {
    pub(super) id: &'static str,
    pub(super) slug: Option<&'static str>,
    pub(super) ticket_type: &'static str,
    pub(super) title: &'static str,
    pub(super) status: &'static str,
    pub(super) closed: bool,
    pub(super) blocked_by: Option<&'static str>,
    pub(super) minutes_old: Option<i64>,
}

pub(super) fn item(spec: &Spec) -> ItemDto {
    let state = match (spec.closed, spec.blocked_by) {
        (true, _) => ReadinessState::Closed,
        (false, Some(_)) => ReadinessState::Blocked,
        (false, None) => ReadinessState::Ready,
    };
    ItemDto {
        id: Some(spec.id.to_owned()),
        kind: ItemDtoKind::Ticket,
        path: format!(".manyhands/tickets/{}/ticket.md", spec.id),
        title: Some(spec.title.to_owned()),
        ticket_type: Some(spec.ticket_type.to_owned()),
        status: Some(spec.status.to_owned()),
        project: None,
        team: None,
        closure: Some(ClosureDto {
            state: if spec.closed {
                ClosureState::Closed
            } else {
                ClosureState::Open
            },
            closed_at: None,
            closed_by: None,
        }),
        slug: spec.slug.map(str::to_owned),
        parent: None,
        deps: Vec::new(),
        readiness: Some(ReadinessDto {
            state,
            reasons: spec
                .blocked_by
                .map(|blocker| ReadinessReasonDto {
                    code: ReadinessReasonCode::OpenDependency,
                    ids: vec![blocker.to_owned()],
                    complete: true,
                })
                .into_iter()
                .collect(),
        }),
        unknown_metadata: serde_json::Map::new(),
        body: None,
        source: None,
        observation: None,
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/repo".to_owned(),
            head_oid: None,
        },
        changed_at: spec.minutes_old.map(ago),
        change_source: None,
        problems: Vec::new(),
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

pub(super) fn nonconforming() -> TicketRow {
    let mut broken = item(&Spec {
        id: ID_A,
        slug: None,
        ticket_type: "",
        title: "",
        status: "",
        closed: false,
        blocked_by: None,
        minutes_old: None,
    });
    broken.id = None;
    broken.path = ".manyhands/tickets/X/ticket.md".to_owned();
    broken.title = None;
    broken.ticket_type = None;
    broken.status = None;
    broken.slug = None;
    broken.closure = None;
    broken.readiness = None;
    broken.problems = vec![ProblemDto {
        code: ProblemCode::MalformedFrontMatter,
        path: None,
        target_id: None,
    }];
    TicketRow::from_item(&broken)
}

pub(super) fn a() -> Spec {
    Spec {
        id: ID_A,
        slug: Some("vh-aaaaa"),
        ticket_type: "cycle",
        title: "Dock layout",
        status: "in progress",
        closed: false,
        blocked_by: None,
        minutes_old: Some(60),
    }
}

/// `a`, `e`, `d`, `c`, `b` and `n`, in the library's order.
pub(super) fn library() -> Vec<TicketRow> {
    let specs = [
        a(),
        Spec {
            id: ID_E,
            slug: Some("vh-eeeee"),
            ticket_type: "task",
            title: "Echo",
            status: "closed",
            minutes_old: Some(90),
            ..a()
        },
        Spec {
            id: ID_D,
            slug: None,
            ticket_type: "defect",
            title: "zeta",
            status: "Ready",
            minutes_old: Some(2 * 24 * 60),
            ..a()
        },
        Spec {
            id: ID_C,
            slug: Some("vh-aaaaa"),
            ticket_type: "feature",
            title: "Old dock",
            status: "done",
            closed: true,
            minutes_old: Some(3 * 24 * 60),
            ..a()
        },
        Spec {
            id: ID_B,
            slug: Some("vh-bbbbb"),
            ticket_type: "task",
            title: "Footer toggles",
            status: "",
            blocked_by: Some(ID_A),
            minutes_old: None,
            ..a()
        },
    ];
    let mut rows: Vec<TicketRow> = specs
        .iter()
        .map(|spec| TicketRow::from_item(&item(spec)))
        .collect();
    rows.push(nonconforming());
    rows
}

pub(super) fn id_of(row: &TicketRow) -> &'static str {
    let id = row.id.as_ref().map(ToString::to_string);
    match id.as_deref() {
        Some(ID_A) => "a",
        Some(ID_B) => "b",
        Some(ID_C) => "c",
        Some(ID_D) => "d",
        Some(ID_E) => "e",
        Some(other) => panic!("{other}"),
        None => "n",
    }
}

pub(super) fn query(
    closure: ClosureFilter,
    readiness: Option<ReadinessFilter>,
    text: &str,
) -> TicketQuery {
    TicketQuery {
        closure,
        readiness,
        text: text.to_owned(),
        sort: TicketSort::default(),
    }
}

pub(super) fn sorted(column: SortColumn, ascending: bool) -> TicketQuery {
    TicketQuery {
        sort: TicketSort { column, ascending },
        ..TicketQuery::default()
    }
}

pub(super) fn tooltip_row(spec: &Spec, context: RowContext, reasons: &[&str]) -> TicketRow {
    let mut row = TicketRow::from_item(&item(spec));
    row.context = context;
    row.reasons = reasons.iter().map(|reason| (*reason).to_owned()).collect();
    row
}

pub(super) fn context(kind: ItemContextKind, branch: Option<&str>, worktree: &str) -> RowContext {
    RowContext {
        kind,
        branch: branch.map(str::to_owned),
        worktree: worktree.to_owned(),
    }
}
