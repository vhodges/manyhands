//! The relationships panel over real repositories: each group and its
//! rows, opening each related ticket in a tab by click and by keyboard,
//! an unresolved dependency that cannot be opened, a relationship
//! problem, no panel on a document, and the panel read again in place
//! when its repository changes.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{repository::ItemDtoKind, results::ProblemCode};

use super::super::tickets::{LIFECYCLE_CLOSED, LIFECYCLE_OPEN};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        relationships::{
            CANNOT_OPEN, CHILDREN, DEPENDENCIES, DEPENDENTS, NONE, PARENT, PROBLEMS, RESOLVED,
            UNRESOLVED,
        },
    },
    services::{ReadKind, requests::Lane},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        workspace::Region,
    },
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const DEP_OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";
const DEP_CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK2";
const PARENT_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK3";
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK4";
const CHILD_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK5";
const CHILD_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK6";
const LATE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK7";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK8";
const MISSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK9";

/// One repository, `a`, built by `build` and refreshed, in an open
/// window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// Two dependencies of `MAIN`, one open and one closed, its parent, one
/// dependent and two children, each made through the library but the
/// closed one, which no local operation makes.
fn graph(fixture: &Fixture, root: &Path) {
    fixture.create_ticket(root, DEP_OPEN, "Open dependency", "open");
    fixture.commit_closed_ticket(root, DEP_CLOSED, "Closed dependency");
    fixture.create_ticket(root, PARENT_ID, "The parent", "open");
    fixture.create_related_ticket(
        root,
        MAIN,
        "Main",
        "open",
        &[DEP_OPEN, DEP_CLOSED],
        Some(PARENT_ID),
    );
    fixture.create_ticket_with_deps(root, DEPENDENT, "The dependent", "open", &[MAIN]);
    fixture.create_related_ticket(root, CHILD_A, "Child A", "open", &[], Some(MAIN));
    fixture.create_related_ticket(root, CHILD_B, "Child B", "open", &[], Some(MAIN));
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn opened(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

/// The short code the library gave `id`.
fn slug(shell: &Shell, root: &Path, id: &str) -> Option<String> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap().slug
}

/// E-23: opens the shown tab's details sheet with All details, by click.
fn details(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("chip-all", cx);
}

fn cell(row: &str, ix: usize, name: &str) -> String {
    format!("rel-{row}-{ix}-{name}")
}

/// Each group says what it is and lists its tickets, each row with the
/// short code and title the library gave, its lifecycle and its
/// resolution in text. Activating each row opens its ticket in a tab, by
/// ID, in the same repository.
#[gpui_kit::test]
fn each_group_lists_its_tickets_and_each_row_opens_its_ticket(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, graph);
    open(&shell, ticket(&root, MAIN), cx);
    details(&shell, cx);

    for (group, name) in [
        ("rel-dependencies", DEPENDENCIES),
        ("rel-dependents", DEPENDENTS),
        ("rel-parent-group", PARENT),
        ("rel-children", CHILDREN),
    ] {
        assert_eq!(shell.label(group, cx).as_deref(), Some(name));
    }

    // Children come in the ticket list's order; each other group in its own.
    let expect = |row: &str, ix: usize, id: &str, title: &str, lifecycle: &'static str| {
        (
            row.to_owned(),
            ix,
            id.to_owned(),
            title.to_owned(),
            lifecycle,
        )
    };
    let mut children: Vec<(String, String)> = (0..2)
        .map(|ix| {
            (
                shell.label(cell("child", ix, "id"), cx).unwrap(),
                shell.label(cell("child", ix, "title"), cx).unwrap(),
            )
        })
        .collect();
    children.sort();
    assert_eq!(
        children,
        [
            (CHILD_A.to_owned(), "Child A".to_owned()),
            (CHILD_B.to_owned(), "Child B".to_owned())
        ]
    );
    // As listed, in the ticket list's order.
    let child_ids: Vec<String> = (0..2)
        .map(|ix| shell.label(cell("child", ix, "id"), cx).unwrap())
        .collect();
    let title_of = |id: &str| if id == CHILD_A { "Child A" } else { "Child B" };
    let rows = [
        expect("dependency", 0, DEP_OPEN, "Open dependency", LIFECYCLE_OPEN),
        expect(
            "dependency",
            1,
            DEP_CLOSED,
            "Closed dependency",
            LIFECYCLE_CLOSED,
        ),
        expect("dependent", 0, DEPENDENT, "The dependent", LIFECYCLE_OPEN),
        expect("parent", 0, PARENT_ID, "The parent", LIFECYCLE_OPEN),
        expect(
            "child",
            0,
            &child_ids[0],
            title_of(&child_ids[0]),
            LIFECYCLE_OPEN,
        ),
        expect(
            "child",
            1,
            &child_ids[1],
            title_of(&child_ids[1]),
            LIFECYCLE_OPEN,
        ),
    ];
    assert_eq!(shell.rows("rel-dependency", cx).len(), 2);
    assert_eq!(shell.rows("rel-dependent", cx).len(), 1);
    assert_eq!(shell.rows("rel-parent", cx).len(), 1);
    assert_eq!(shell.rows("rel-child", cx).len(), 2);
    for (row, ix, id, title, lifecycle) in &rows {
        let ix = *ix;
        assert_eq!(
            shell.label(cell(row, ix, "id"), cx).as_deref(),
            Some(id.as_str())
        );
        assert_eq!(
            shell.label(cell(row, ix, "title"), cx).as_deref(),
            Some(title.as_str())
        );
        let code = slug(&shell, &root, id);
        assert_eq!(shell.label(cell(row, ix, "slug"), cx), code);
        assert_eq!(
            shell.label(cell(row, ix, "lifecycle"), cx).as_deref(),
            Some(*lifecycle)
        );
        assert_eq!(
            shell.label(cell(row, ix, "resolution"), cx).as_deref(),
            Some(RESOLVED)
        );
        assert!(!shell.present(cell(row, ix, "cannot-open"), cx));
    }
    assert!(!shell.present("rel-problems", cx));

    // Each row opens its ticket in a tab; the main tab stays open.
    let row_name = |row: &str| match row {
        "dependency" => "rel-dependency",
        "dependent" => "rel-dependent",
        "parent" => "rel-parent",
        _ => "rel-child",
    };
    for (row, ix, id, _, _) in &rows {
        open(&shell, ticket(&root, MAIN), cx);
        assert_eq!(opened(&shell, cx), Some(ticket(&root, MAIN)));
        details(&shell, cx);
        shell.click((row_name(row), *ix), cx);
        assert_eq!(opened(&shell, cx), Some(ticket(&root, id)), "{row} {ix}");
        details(&shell, cx);
        assert_eq!(shell.label("item-id", cx).as_deref(), Some(id.as_str()));
    }
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len());
    assert_eq!(tabs, 1 + rows.len());
}

/// A group with no tickets says so, as a ticket with no parent does.
#[gpui_kit::test]
fn an_empty_group_says_none(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, MAIN, "Alone", "open");
    });
    open(&shell, ticket(&root, MAIN), cx);
    details(&shell, cx);
    for group in ["dependency", "dependent", "parent", "child"] {
        assert_eq!(
            shell.label(format!("rel-{group}-none"), cx).as_deref(),
            Some(NONE),
            "{group}"
        );
    }
    assert!(shell.rows("rel-dependency", cx).is_empty());
}

/// The panel is reachable from the keyboard: from the content region,
/// Tab reaches a row and Enter opens its ticket.
#[gpui_kit::test]
fn a_row_opens_its_ticket_from_the_keyboard(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, graph);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-3", cx);
    details(&shell, cx);
    let focused = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window
                .try_find(("rel-dependent", 0usize))
                .and_then(|row| row.focused())
                == Some(true)
        })
    };
    let in_content = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window.find(Region::Content.id()).focused() == Some(true)
        })
    };
    let mut presses = 0;
    while !focused(&shell, cx) {
        presses += 1;
        assert!(presses < 60, "Tab never reached the dependent's row");
        shell.keys("tab", cx);
        assert!(
            in_content(&shell, cx),
            "Tab stays within the content region"
        );
    }
    shell.press("enter", cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, DEPENDENT)));
    // The row went with its tab; focus falls back to the content region.
    assert!(in_content(&shell, cx));
}

/// An unresolved dependency shows the ID it names and says, in text, that
/// it cannot be opened: activating it opens nothing, and Tab does not
/// stop on it.
#[gpui_kit::test]
fn an_unresolved_dependency_shows_its_id_and_cannot_be_opened(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket_with_deps(root, MAIN, "Waits", "open", &[MISSING]);
    });
    open(&shell, ticket(&root, MAIN), cx);
    details(&shell, cx);
    assert_eq!(
        shell.label(cell("dependency", 0, "id"), cx).as_deref(),
        Some(MISSING)
    );
    assert_eq!(
        shell
            .label(cell("dependency", 0, "resolution"), cx)
            .as_deref(),
        Some(UNRESOLVED)
    );
    assert_eq!(
        shell
            .label(cell("dependency", 0, "cannot-open"), cx)
            .as_deref(),
        Some(CANNOT_OPEN)
    );
    assert!(!shell.present(cell("dependency", 0, "title"), cx));
    assert!(!shell.present(cell("dependency", 0, "slug"), cx));
    assert!(!shell.present(cell("dependency", 0, "lifecycle"), cx));
    let focusable = shell.frame(cx, |window, _| {
        window
            .try_find(("rel-dependency", 0usize))
            .and_then(|row| row.focused())
    });
    assert_eq!(focusable, None, "no focus is offered");

    shell.click(("rel-dependency", 0usize), cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, MAIN)));
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len());
    assert_eq!(tabs, 1);
}

/// A problem with the ticket's `deps` is listed under the groups, with
/// its guidance and the ID it is about, and not again among the item's
/// other problems.
#[gpui_kit::test]
fn a_relationship_problem_is_listed_under_the_groups(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(
            root,
            MAIN,
            "Repeats",
            &format!("slug: Not-A-Slug\ndeps:\n  - {DEP_OPEN}\n  - {DEP_OPEN}\n"),
        );
        fixture.commit_ticket(root, DEP_OPEN, "Open dependency", "");
    });
    open(&shell, ticket(&root, MAIN), cx);
    details(&shell, cx);
    assert_eq!(shell.label("rel-problems", cx).as_deref(), Some(PROBLEMS));
    assert_eq!(
        shell.rows("rel-problem", cx),
        [ProblemCode::DuplicateDependency.as_str()]
    );
    assert_eq!(
        shell.rows("rel-problem-guidance", cx),
        [ProblemCode::DuplicateDependency.guidance()]
    );
    assert_eq!(
        shell.label(("rel-problem-target", 0usize), cx).as_deref(),
        Some(DEP_OPEN)
    );
    // The repeat counts once.
    assert_eq!(shell.rows("rel-dependency", cx).len(), 1);
    // The other problem stays with the item's.
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::InvalidSlug.as_str()]
    );
}

/// A document's tab has no relationships panel.
#[gpui_kit::test]
fn a_document_tab_shows_no_relationships_panel(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, "docs/guide.md", "Guide");
    });
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    details(&shell, cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Guide"));
    assert!(!shell.present("relationships", cx));
    assert_eq!(shell.backend.reads_seen(ReadKind::Dependencies), 0);
    assert_eq!(shell.backend.reads_seen(ReadKind::Children), 0);
}

/// Review Focus 1: when the repository changes, the tab reads its
/// relationships again in place, once, with its item: a ticket that now
/// depends on it is listed.
#[gpui_kit::test]
async fn a_refresh_reads_the_relationships_again_in_place(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, graph);
    open(&shell, ticket(&root, MAIN), cx);
    details(&shell, cx);
    assert_eq!(shell.rows("rel-dependent", cx).len(), 1);
    let before =
        [ReadKind::Dependencies, ReadKind::Children].map(|kind| shell.backend.reads_seen(kind));

    shell
        .fixture
        .create_ticket_with_deps(&root, LATE, "Came later", "open", &[MAIN]);
    shell.select(&root, cx);
    // R-60 refuses `secondary-r` behind the sheet.
    shell.keys("escape", cx);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| request.lane == Lane::User)[0];
    shell.until_ended(refresh, cx).await;
    shell.run(cx);
    details(&shell, cx);

    let after =
        [ReadKind::Dependencies, ReadKind::Children].map(|kind| shell.backend.reads_seen(kind));
    assert_eq!(after, before.map(|seen| seen + 1), "one read each");
    assert_eq!(opened(&shell, cx), Some(ticket(&root, MAIN)));
    let mut dependents: Vec<String> = (0..2)
        .map(|ix| shell.label(cell("dependent", ix, "id"), cx).unwrap())
        .collect();
    dependents.sort();
    assert_eq!(dependents, [DEPENDENT, LATE]);
}

/// A focused row's focus follows its ticket, not its place: a reload that
/// reorders its group leaves the same ticket's row focused, and Enter
/// opens that ticket.
#[gpui_kit::test]
fn a_focused_row_keeps_its_ticket_when_a_reload_reorders_its_group(cx: &mut TestAppContext) {
    let deps = |first: &str, second: &str| format!("deps:\n  - {first}\n  - {second}\n");
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, DEP_OPEN, "First", "");
        fixture.commit_ticket(root, LATE, "Second", "");
        fixture.commit_ticket(root, MAIN, "Main", &deps(DEP_OPEN, LATE));
    });
    open(&shell, ticket(&root, MAIN), cx);
    let focused = |shell: &Shell, ix: usize, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window
                .try_find(("rel-dependency", ix))
                .and_then(|row| row.focused())
                == Some(true)
        })
    };
    shell.keys("secondary-3", cx);
    details(&shell, cx);
    let mut presses = 0;
    while !focused(&shell, 0, cx) {
        presses += 1;
        assert!(presses < 60, "Tab never reached the first dependency");
        shell.keys("tab", cx);
    }
    assert_eq!(
        shell.label(cell("dependency", 0, "id"), cx).as_deref(),
        Some(DEP_OPEN)
    );

    shell
        .fixture
        .commit_ticket(&root, MAIN, "Main", &deps(LATE, DEP_OPEN));
    shell.fixture.refresh(&root);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .unwrap();
    pane.update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    assert_eq!(
        shell.label(cell("dependency", 0, "id"), cx).as_deref(),
        Some(LATE)
    );
    assert!(focused(&shell, 1, cx), "focus moved with its ticket");
    assert!(!focused(&shell, 0, cx));

    shell.press("enter", cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, DEP_OPEN)));
}
