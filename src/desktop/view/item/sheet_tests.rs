//! The details sheet while viewing, over real repositories: each chip and
//! All details open it on their field, it is read only, its rows are the
//! design's for a ticket and a document, other fields keep the library's
//! order, the four relationship groups are in it (R-54), Escape, Done and
//! the close button give focus back to the chip, no workspace key acts
//! behind it (R-60), its Inspect link opens the item's Inspect tab, it
//! closes with its tab, a refresh updates it in place, and it writes
//! nothing. An Inspect tab of a conforming item is not called
//! nonconforming.

use std::path::{Path, PathBuf};

use gpui_kit::{ElementId, Entity, TestAppContext, component::WindowExt, test::TestWindowExt};
use manyhands::{
    repository::{ItemDto, ItemDtoKind},
    results::ProblemCode,
};

use super::{
    ItemPane, NONCONFORMING_FILE,
    chips::ChipField,
    sheet::{
        DOCUMENT_DETAILS, DONE, FACTS, INSPECT, KEPT, NOT_CLOSED, OTHER_FIELDS, READ_ONLY,
        TICKET_DETAILS, VIEWING,
    },
};
use crate::desktop::{
    model::{
        item::{KIND_PRIMARY, NOT_REPRESENTABLE, metadata_value},
        navigation::OpenTarget,
        relationships::{CHILDREN, DEPENDENCIES, DEPENDENTS, NONE, PARENT, RELATIONSHIPS},
    },
    services::{
        JobKind,
        requests::{Intent, Lane, RequestState},
    },
    test_support::{
        fixtures::{Fixture, item_id, ticket_path},
        snapshot::RepositorySnapshot,
    },
    view::{
        harness::{Shell, key},
        settings::Shown,
        tickets::{LIFECYCLE_CLOSED, LIFECYCLE_OPEN},
        workspace::QuitPhase,
    },
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR0";
const PARENT_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const BLOCKER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const DONE_DEP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const CHILD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR7";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";
const PLAIN_DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS1";
const SLUG: &str = "ae-k9x2b";
const PARENT_SLUG: &str = "mh-vh-w03xy";
const MARKER: &str = "docs/marker.md";

/// A managed document's source at `id`, with `extra` front matter lines.
fn document_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n\
         {extra}---\nBody of {title}.\n"
    )
}

/// One repository, `a`, built by `build` and refreshed, in an open
/// window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn document(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(id),
    }
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

fn opened(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

fn tabs(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len())
}

/// Whether a details sheet is open in the window.
fn has_sheet(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_sheet(cx))
}

fn focused(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the element `id` is drawn inside `within`.
fn inside(
    shell: &Shell,
    id: impl Into<ElementId>,
    within: &'static str,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.path().contains(&ElementId::from(within)))
    })
}

/// Presses Tab until the element `id` holds focus.
fn tab_to(shell: &Shell, id: &'static str, cx: &mut TestAppContext) {
    let mut presses = 0;
    while !focused(shell, id, cx) {
        presses += 1;
        assert!(presses < 40, "Tab never reached {id}");
        shell.keys("tab", cx);
    }
}

/// A ticket with every chip: its type, status, project, team, a parent
/// with a short code, and an open dependency it waits on; and a document
/// whose status, author and date are strings.
fn every_chip(fixture: &Fixture, root: &Path) {
    fixture.commit_ticket(root, PARENT_ID, "Parent", &format!("slug: {PARENT_SLUG}\n"));
    fixture.create_ticket(root, BLOCKER, "Blocker", "open");
    fixture.commit_ticket(
        root,
        MAIN,
        "Main",
        &format!("project: p\nteam: t\nparent: {PARENT_ID}\ndeps: [{BLOCKER}]\n"),
    );
    fixture.commit_file(
        root,
        "docs/notes/kept.md",
        document_source(
            DOCUMENT,
            "Kept",
            "status: draft\nauthor: Ada Lovelace\ndate: 2026-10-05\n",
        )
        .as_bytes(),
    );
}

/// Each chip, and All details, opens the sheet with its own field
/// focused, by click and by keyboard, for a ticket and for a document.
#[gpui_kit::test]
fn each_chip_and_all_details_open_the_sheet_and_focus_its_field(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, every_chip);

    open(&shell, ticket(&root, MAIN), cx);
    for (field, row) in [
        (ChipField::Type, "sheet-type"),
        (ChipField::Status, "sheet-status"),
        (ChipField::Project, "sheet-project"),
        (ChipField::Team, "sheet-team"),
        (ChipField::Parent, "sheet-parent"),
        (ChipField::WaitingOn, "sheet-waiting-on"),
        (ChipField::All, "sheet-title"),
    ] {
        assert!(!has_sheet(&shell, cx), "{field:?}: no sheet before");
        shell.click(field.id(), cx);
        assert!(has_sheet(&shell, cx), "{field:?} opens the sheet");
        assert_eq!(
            shell.label("sheet-heading", cx).as_deref(),
            Some(TICKET_DETAILS)
        );
        assert!(focused(&shell, row, cx), "{field:?} focuses {row}");
        shell.keys("escape", cx);
    }

    // By keyboard: Tab to a chip, then Enter.
    shell.keys("secondary-3", cx);
    tab_to(&shell, ChipField::Status.id(), cx);
    shell.press("enter", cx);
    assert!(has_sheet(&shell, cx));
    assert!(focused(&shell, "sheet-status", cx));
    shell.keys("escape", cx);

    open(&shell, document(&root, DOCUMENT), cx);
    for (field, row) in [
        (ChipField::Status, "sheet-status"),
        (ChipField::Author, "sheet-author"),
        (ChipField::Date, "sheet-date"),
        (ChipField::Updated, "sheet-changed"),
        (ChipField::All, "sheet-title"),
    ] {
        shell.click(field.id(), cx);
        assert!(has_sheet(&shell, cx), "{field:?} opens the sheet");
        assert_eq!(
            shell.label("sheet-heading", cx).as_deref(),
            Some(DOCUMENT_DETAILS)
        );
        assert!(focused(&shell, row, cx), "{field:?} focuses {row}");
        shell.keys("escape", cx);
    }
}

/// While viewing, the sheet says so in its first line and its footer, and
/// nothing typed, deleted, cut or pasted into a focused field changes it,
/// the header or the repository.
#[gpui_kit::test]
fn the_sheet_is_read_only_while_viewing(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Kept", "project: p\n");
        fixture.commit_file(
            root,
            "docs/kept.md",
            document_source(DOCUMENT, "Kept document", "").as_bytes(),
        );
    });
    let before = RepositorySnapshot::take(&root);
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    assert_eq!(
        shell.label("sheet-description", cx).as_deref(),
        Some(VIEWING)
    );
    assert_eq!(shell.label("sheet-note", cx).as_deref(), Some(READ_ONLY));
    assert_eq!(shell.label("sheet-done", cx).as_deref(), Some(DONE));
    assert!(focused(&shell, "sheet-title", cx));

    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("pasted".to_owned()));
    shell.keys("x y z backspace delete secondary-x secondary-v", cx);
    shell.keys("tab", cx);
    assert!(focused(&shell, "sheet-type", cx));
    shell.keys("a b c backspace secondary-v", cx);
    assert!(has_sheet(&shell, cx));
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Kept")
    );
    assert_eq!(shell.label("item-type", cx).as_deref(), Some("task"));
    assert_eq!(shell.label("item-project", cx).as_deref(), Some("p"));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Kept"));
    shell.keys("escape", cx);

    open(&shell, document(&root, DOCUMENT), cx);
    shell.click(ChipField::All.id(), cx);
    assert_eq!(
        shell.label("sheet-description", cx).as_deref(),
        Some(VIEWING)
    );
    assert_eq!(shell.label("sheet-note", cx).as_deref(), Some(READ_ONLY));
    shell.keys("q w backspace secondary-v", cx);
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Kept document")
    );
    assert_eq!(RepositorySnapshot::take(&root), before);
}

/// A ticket's sheet has every row of the design, in its own field or
/// box: Title, Type and Status (required), Project, Team, Parent, Waiting
/// on with the readiness note, Dependents, Children, Other fields kept as
/// written, the Facts, the Problems and the Inspect link; and a closed
/// ticket says who closed it and when.
#[gpui_kit::test]
fn ticket_details_show_every_row_of_the_design(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PARENT_ID, "Parent", &format!("slug: {PARENT_SLUG}\n"));
        fixture.commit_closed_ticket(root, DONE_DEP, "Done dependency");
        fixture.commit_ticket(
            root,
            MAIN,
            "Main",
            &format!(
                "slug: {SLUG}\nproject: p\nteam: t\nparent: {PARENT_ID}\ndeps: [{DONE_DEP}]\n\
                 tagged: !secret hidden\nzeta: last\n"
            ),
        );
        fixture.create_ticket_with_deps(root, DEPENDENT, "Dependent", "open", &[MAIN]);
        fixture.create_related_ticket(root, CHILD, "Child", "open", &[], Some(MAIN));
        fixture.commit_closed_ticket(root, CLOSED, "Closed one");
    });
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    assert_eq!(
        shell.label("sheet-heading", cx).as_deref(),
        Some(TICKET_DETAILS)
    );
    assert_eq!(
        shell.label("sheet-description", cx).as_deref(),
        Some(VIEWING)
    );

    for (row, name) in [
        ("sheet-title", "Title, required"),
        ("sheet-type", "Type, required"),
        ("sheet-status", "Status, required"),
        ("sheet-project", "Project"),
        ("sheet-team", "Team"),
        ("sheet-parent", "Parent"),
        ("sheet-waiting-on", "Waiting on"),
        ("sheet-dependents", "Dependents"),
        ("sheet-children", "Children"),
        ("sheet-other", OTHER_FIELDS),
        ("sheet-facts", FACTS),
        ("sheet-problems", "Problems"),
    ] {
        assert_eq!(shell.label(row, cx).as_deref(), Some(name), "{row}");
        assert!(inside(&shell, row, "sheet-content", cx), "{row}");
    }
    for (id, value) in [
        ("sheet-title-value", "Main"),
        ("item-type", "task"),
        ("item-status", "open"),
        ("item-project", "p"),
        ("item-team", "t"),
    ] {
        assert_eq!(shell.label(id, cx).as_deref(), Some(value), "{id}");
    }

    // Parent and Waiting on hold D1's rows, with the readiness note.
    assert_eq!(
        shell.label("rel-parent-0-slug", cx).as_deref(),
        Some(PARENT_SLUG)
    );
    assert!(inside(&shell, ("rel-parent", 0usize), "sheet-parent", cx));
    assert_eq!(
        shell.label("rel-dependency-0-id", cx).as_deref(),
        Some(DONE_DEP)
    );
    assert!(inside(
        &shell,
        ("rel-dependency", 0usize),
        "sheet-waiting-on",
        cx
    ));
    assert_eq!(
        shell.label("sheet-readiness", cx),
        Some(format!(
            "{DONE_DEP} is closed, so this ticket is ready to work on."
        ))
    );
    assert_eq!(
        shell.label("rel-dependent-0-id", cx).as_deref(),
        Some(DEPENDENT)
    );
    assert!(inside(
        &shell,
        ("rel-dependent", 0usize),
        "sheet-dependents",
        cx
    ));
    assert_eq!(shell.label("rel-child-0-id", cx).as_deref(), Some(CHILD));
    assert!(inside(&shell, ("rel-child", 0usize), "sheet-children", cx));

    // Other fields, as written, in the library's order.
    assert_eq!(shell.label("sheet-other-note", cx).as_deref(), Some(KEPT));
    assert_eq!(shell.rows("meta-key", cx), ["tagged", "zeta"]);
    assert_eq!(shell.rows("meta-value", cx), [NOT_REPRESENTABLE, "last"]);

    // The Facts: short code and ID with Copy, D1's kind and lifecycle,
    // closure, provenance and the file.
    let item = shown(&shell, &root, MAIN);
    for (id, value) in [
        ("item-kind", "Ticket".to_owned()),
        ("item-slug", SLUG.to_owned()),
        ("item-id", MAIN.to_owned()),
        ("item-lifecycle", LIFECYCLE_OPEN.to_owned()),
        ("sheet-closure", NOT_CLOSED.to_owned()),
        ("provenance-kind", KIND_PRIMARY.to_owned()),
        ("provenance-branch", "main".to_owned()),
        ("provenance-worktree", key(&root)),
        ("item-path", ticket_path(MAIN)),
        ("item-changed", item.changed_at.clone().unwrap()),
    ] {
        assert_eq!(shell.label(id, cx), Some(value), "{id}");
        assert!(inside(&shell, id, "sheet-facts", cx), "{id}");
    }
    for copy in ["copy-slug", "copy-id"] {
        assert!(inside(&shell, copy, "sheet-facts", cx), "{copy}");
    }
    assert!(!shell.present("item-closed-at", cx));

    // The item's own problems, and the link to the whole file.
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::MetadataNotRepresentable.as_str()]
    );
    assert!(inside(
        &shell,
        ("item-problem", 0usize),
        "sheet-problems",
        cx
    ));
    assert_eq!(shell.label("sheet-inspect", cx).as_deref(), Some(INSPECT));
    assert_eq!(shell.label("sheet-note", cx).as_deref(), Some(READ_ONLY));
    assert_eq!(shell.label("sheet-done", cx).as_deref(), Some(DONE));

    // A closed ticket says who closed it and when.
    open(&shell, ticket(&root, CLOSED), cx);
    shell.click(ChipField::All.id(), cx);
    let closure = shown(&shell, &root, CLOSED).closure.unwrap();
    let (by, at) = (closure.closed_by.unwrap(), closure.closed_at.unwrap());
    assert_eq!(
        shell.label("sheet-closure", cx),
        Some(format!("Closed by {by} on {at}"))
    );
    assert_eq!(shell.label("item-closed-by", cx), Some(by));
    assert_eq!(shell.label("item-closed-at", cx), Some(at));
    assert_eq!(
        shell.label("item-lifecycle", cx).as_deref(),
        Some(LIFECYCLE_CLOSED)
    );
}

/// A document's sheet shows its Status, Date and Author from its other
/// keys exactly as the library gives them, its folder, the remaining keys
/// as Other fields, or None, and its Facts; it has no ticket rows and no
/// relationships.
#[gpui_kit::test]
fn document_details_show_status_date_and_author_as_the_library_gives_them(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(
            root,
            "docs/notes/kept.md",
            document_source(
                DOCUMENT,
                "Kept",
                "status: draft\ndate: 2026-10-05\n\
                 author: \"Ada && Bob <ada@example.invalid>\"\nreviewers: [ada, bob]\n",
            )
            .as_bytes(),
        );
        fixture.commit_file(
            root,
            "docs/plain.md",
            document_source(PLAIN_DOCUMENT, "Plain", "status: 3\n").as_bytes(),
        );
    });
    open(&shell, document(&root, DOCUMENT), cx);
    shell.click(ChipField::All.id(), cx);
    assert_eq!(
        shell.label("sheet-heading", cx).as_deref(),
        Some(DOCUMENT_DETAILS)
    );
    let item = shown(&shell, &root, DOCUMENT);
    for (row, name, id, field) in [
        ("sheet-status", "Status", "item-status", "status"),
        ("sheet-date", "Date", "item-date", "date"),
        ("sheet-author", "Author", "item-author", "author"),
    ] {
        assert_eq!(shell.label(row, cx).as_deref(), Some(name), "{row}");
        assert_eq!(
            shell.label(id, cx),
            metadata_value(&item.unknown_metadata[field]),
            "{id}"
        );
    }
    assert_eq!(
        shell.label("item-author", cx).as_deref(),
        Some("Ada && Bob <ada@example.invalid>")
    );
    assert_eq!(
        shell.label("sheet-title", cx).as_deref(),
        Some("Title, required")
    );
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Kept")
    );
    assert_eq!(shell.label("sheet-folder", cx).as_deref(), Some("Folder"));
    assert_eq!(
        shell.label("item-folder", cx).as_deref(),
        Some("docs/notes")
    );
    assert_eq!(shell.rows("meta-key", cx), ["reviewers"]);
    assert_eq!(shell.rows("meta-value", cx), [r#"["ada","bob"]"#]);
    for (id, value) in [
        ("item-kind", "Document"),
        ("item-id", DOCUMENT),
        ("item-file-name", "kept.md"),
        ("provenance-kind", KIND_PRIMARY),
    ] {
        assert_eq!(shell.label(id, cx).as_deref(), Some(value), "{id}");
        assert!(inside(&shell, id, "sheet-facts", cx), "{id}");
    }
    assert!(inside(&shell, "copy-id", "sheet-facts", cx));
    assert_eq!(shell.label("sheet-inspect", cx).as_deref(), Some(INSPECT));
    for id in [
        "relationships",
        "sheet-type",
        "sheet-parent",
        "sheet-waiting-on",
        "item-lifecycle",
        "copy-slug",
    ] {
        assert!(!shell.present(id, cx), "{id}");
    }

    // A status that is not text is shown as the library's JSON; a key the
    // file does not give has no value; no other key reads None.
    open(&shell, document(&root, PLAIN_DOCUMENT), cx);
    shell.click(ChipField::All.id(), cx);
    assert_eq!(shell.label("item-status", cx).as_deref(), Some("3"));
    assert!(shell.present("sheet-date", cx));
    assert!(!shell.present("item-date", cx));
    assert!(!shell.present("item-author", cx));
    assert!(shell.rows("meta-key", cx).is_empty());
    assert_eq!(shell.label("sheet-other-none", cx).as_deref(), Some(NONE));
}

/// Other fields are listed by key in the library's order, each value as
/// the library's JSON or text, and none of them can be edited: none takes
/// focus, and clicking or typing changes nothing.
#[gpui_kit::test]
fn other_fields_are_listed_in_the_librarys_key_order_and_are_not_editable(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(
            root,
            MAIN,
            "Odd",
            "zeta: last\nalpha: [1, 2]\nmiddle:\n  k: v\n",
        );
    });
    let before = RepositorySnapshot::take(&root);
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    let item = shown(&shell, &root, MAIN);
    let keys: Vec<&str> = item.unknown_metadata.keys().map(String::as_str).collect();
    assert_eq!(keys, ["alpha", "middle", "zeta"]);
    assert_eq!(shell.rows("meta-key", cx), keys);
    assert_eq!(
        shell.rows("meta-value", cx),
        ["[1,2]", r#"{"k":"v"}"#, "last"]
    );
    assert_eq!(shell.label("sheet-other-note", cx).as_deref(), Some(KEPT));
    assert!(inside(&shell, ("meta-key", 0usize), "sheet-other", cx));

    for ix in 0..keys.len() {
        for name in ["meta-key", "meta-value"] {
            let focusable = shell.frame(cx, |window, _| window.find((name, ix)).focused());
            assert_eq!(focusable, None, "{name} {ix} takes no focus");
        }
    }
    shell.click(("meta-value", 0usize), cx);
    shell.keys("x backspace secondary-v", cx);
    assert!(has_sheet(&shell, cx));
    assert_eq!(
        shell.rows("meta-value", cx),
        ["[1,2]", r#"{"k":"v"}"#, "last"]
    );
    assert_eq!(RepositorySnapshot::take(&root), before);
}

/// R-54: the ticket's sheet holds all four relationship groups, each with
/// its row, and the item's page holds none of them.
#[gpui_kit::test]
fn the_four_relationship_groups_are_in_the_ticket_sheet(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, BLOCKER, "Dependency", "open");
        fixture.create_ticket(root, PARENT_ID, "Parent", "open");
        fixture.create_related_ticket(root, MAIN, "Main", "open", &[BLOCKER], Some(PARENT_ID));
        fixture.create_ticket_with_deps(root, DEPENDENT, "Dependent", "open", &[MAIN]);
        fixture.create_related_ticket(root, CHILD, "Child", "open", &[], Some(MAIN));
    });
    open(&shell, ticket(&root, MAIN), cx);
    assert!(!shell.present("relationships", cx), "not on the page");
    shell.click(ChipField::All.id(), cx);
    assert_eq!(
        shell.label("relationships", cx).as_deref(),
        Some(RELATIONSHIPS)
    );
    assert!(inside(&shell, "relationships", "sheet-content", cx));
    for (group, name, row) in [
        ("rel-dependencies", DEPENDENCIES, "rel-dependency"),
        ("rel-dependents", DEPENDENTS, "rel-dependent"),
        ("rel-parent-group", PARENT, "rel-parent"),
        ("rel-children", CHILDREN, "rel-child"),
    ] {
        assert_eq!(shell.label(group, cx).as_deref(), Some(name), "{group}");
        assert!(inside(&shell, group, "relationships", cx), "{group}");
        assert_eq!(shell.rows(row, cx).len(), 1, "{row}");
        assert!(!inside(&shell, (row, 0usize), "item-page", cx), "{row}");
    }
}

/// Escape, Done and the sheet's close button each close it and give focus
/// back to the chip that opened it, whether a click or Enter opened it.
#[gpui_kit::test]
fn escape_done_and_close_return_focus_to_the_chip(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
    });
    open(&shell, ticket(&root, MAIN), cx);

    shell.click(ChipField::Type.id(), cx);
    assert!(has_sheet(&shell, cx));
    shell.keys("escape", cx);
    assert!(!has_sheet(&shell, cx), "Escape closes it");
    assert!(focused(&shell, ChipField::Type.id(), cx));

    shell.click(ChipField::Status.id(), cx);
    assert!(has_sheet(&shell, cx));
    shell.click("sheet-done", cx);
    assert!(!has_sheet(&shell, cx), "Done closes it");
    assert!(focused(&shell, ChipField::Status.id(), cx));

    shell.click(ChipField::All.id(), cx);
    assert!(has_sheet(&shell, cx));
    shell.frame(cx, |window, cx| {
        window.within("sheet-content").click("close", cx)
    });
    shell.run(cx);
    assert!(!has_sheet(&shell, cx), "the close button closes it");
    assert!(focused(&shell, ChipField::All.id(), cx));

    // Opened by Enter, Escape gives focus back to the same chip.
    shell.keys("secondary-3", cx);
    tab_to(&shell, ChipField::Project.id(), cx);
    shell.press("enter", cx);
    assert!(has_sheet(&shell, cx));
    assert!(focused(&shell, "sheet-project", cx));
    shell.keys("escape", cx);
    assert!(!has_sheet(&shell, cx));
    assert!(focused(&shell, ChipField::Project.id(), cx));
}

/// R-60: while the sheet is open, no workspace key acts behind it: the
/// region keys, Find, the tab keys, Refresh and Rebuild, Settings,
/// Drafts and the page keys leave the sheet open and focused, the tabs as
/// they were, and submit nothing.
#[gpui_kit::test]
fn no_workspace_key_acts_behind_the_sheet(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, FIRST, "First", "");
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
    });
    shell.select(&root, cx);
    open(&shell, ticket(&root, FIRST), cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::Status.id(), cx);
    assert!(focused(&shell, "sheet-status", cx));
    let requests = shell.requests(cx, |_| true).len();
    let offset = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.item_offset(cx));

    for key in [
        "f6",
        "shift-f6",
        "secondary-1",
        "secondary-2",
        "secondary-3",
        "secondary-4",
        "secondary-p",
        "ctrl-tab",
        "ctrl-shift-tab",
        "secondary-w",
        "secondary-r",
        "secondary-shift-r",
        "secondary-,",
        "secondary-shift-d",
        "pageup",
        "pagedown",
        "home",
        "end",
    ] {
        shell.keys(key, cx);
        assert!(has_sheet(&shell, cx), "{key}: the sheet stays open");
        assert!(focused(&shell, "sheet-status", cx), "{key}: focus stays");
        assert_eq!(opened(&shell, cx), Some(ticket(&root, MAIN)), "{key}");
        assert_eq!(tabs(&shell, cx), 2, "{key}");
        let (settings, drafts, now) = shell.workspace.read_with(cx, |workspace, cx| {
            (
                workspace.settings_shown(cx),
                workspace.drafts_shown(cx),
                workspace.item_offset(cx),
            )
        });
        assert_eq!(settings, Shown::Tabs, "{key}");
        assert!(!drafts, "{key}");
        assert_eq!(now, offset, "{key}");
        assert_eq!(shell.requests(cx, |_| true).len(), requests, "{key}");
    }
}

/// The Inspect link, at the end of the sheet and brought into view when
/// Tab reaches it, closes the sheet and opens the item's file by its path
/// in an Inspect tab of its own, which shows the whole file.
#[gpui_kit::test]
fn the_inspect_link_opens_the_items_inspect_tab(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "zeta: last\n");
    });
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    tab_to(&shell, "sheet-inspect", cx);
    let shown = shell.frame(cx, |window, _| window.find("sheet-inspect").visible());
    assert!(shown, "the focused link is in view");
    shell.press("enter", cx);

    assert!(!has_sheet(&shell, cx));
    assert_eq!(
        opened(&shell, cx),
        Some(OpenTarget::Inspect {
            repo: key(&root),
            worktree: key(&root),
            path: ticket_path(MAIN),
        })
    );
    assert_eq!(tabs(&shell, cx), 2);
    assert_eq!(shell.label("item-path", cx), Some(ticket_path(MAIN)));
    assert!(shell.present("item-source", cx), "the whole file");
}

/// Closing the sheet's tab by the tab's own close action, called as the
/// close button would (the backdrop takes the click itself), closes the
/// sheet; the tab left shown opens its own sheet.
#[gpui_kit::test]
fn closing_the_tab_by_its_action_closes_the_sheet(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, FIRST, "First", "");
        fixture.commit_ticket(root, MAIN, "Main", "");
    });
    open(&shell, ticket(&root, FIRST), cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    assert!(has_sheet(&shell, cx));

    let id = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.shown_tab())
        .unwrap();
    let workspace = shell.workspace.clone();
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| workspace.close_tab(id, window, cx))
    });
    shell.run(cx);
    assert!(!has_sheet(&shell, cx));
    assert!(!shell.present("sheet-heading", cx));
    assert_eq!(tabs(&shell, cx), 1);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, FIRST)));
    assert!(
        focused(&shell, ("tab", 0usize), cx),
        "focus is on the remaining tab, in the content region"
    );

    shell.click(ChipField::All.id(), cx);
    assert!(has_sheet(&shell, cx));
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("First")
    );
}

/// Quit asked for behind the sheet while a job runs closes the sheet, as
/// it closes a dialog, and Quit now holds focus, in view; the next frame
/// opens nothing over it. Quitting while drafts are saved waits in the same
/// way (`wait_to_quit`).
#[gpui_kit::test]
async fn quit_behind_the_sheet_closes_it_and_focuses_quit_now(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
    });
    open(&shell, ticket(&root, MAIN), cx);
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    let id = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRefresh { root: root.clone() },
            Lane::User,
            None,
            None,
        )
    });
    held.wait_reached();
    shell.click(ChipField::Project.id(), cx);
    assert!(has_sheet(&shell, cx));

    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Finishing(id));
    assert!(!has_sheet(&shell, cx), "the sheet closes");
    assert!(focused(&shell, "quit-now", cx), "Quit now holds focus");
    let shown = shell.frame(cx, |window, _| window.find("quit-now").visible());
    assert!(shown, "Quit now is in view");
    shell.run(cx);
    assert!(!has_sheet(&shell, cx));
    assert!(focused(&shell, "quit-now", cx));

    // The worker must not outlive the test: let the held job end.
    held.release();
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Finished(..))
            )
        })
        .await;
}

/// A read again while the sheet is open updates it in place: it stays
/// open, shows the new values, and keeps its focused field.
#[gpui_kit::test]
fn a_refresh_updates_the_open_sheet_in_place(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Before", "project: p\n");
    });
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::Project.id(), cx);
    assert!(focused(&shell, "sheet-project", cx));

    shell
        .fixture
        .commit_ticket(&root, MAIN, "After", "project: q\n");
    shell.fixture.refresh(&root);
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    assert!(has_sheet(&shell, cx));
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("After")
    );
    assert_eq!(shell.label("item-project", cx).as_deref(), Some("q"));
    assert!(focused(&shell, "sheet-project", cx));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("After"));
}

/// A focused row that a read again removes leaves focus inside the sheet,
/// not behind it, so Escape still closes it and gives focus back to the
/// chip.
#[gpui_kit::test]
fn a_focused_row_that_goes_leaves_focus_in_the_sheet(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, BLOCKER, "Dependency", "open");
        fixture.commit_ticket(root, MAIN, "Main", &format!("deps: [{BLOCKER}]\n"));
    });
    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    let row = ("rel-dependency", 0usize);
    let mut presses = 0;
    while !focused(&shell, row, cx) {
        presses += 1;
        assert!(presses < 40, "Tab never reached the dependency");
        shell.keys("tab", cx);
    }

    shell.fixture.commit_ticket(&root, MAIN, "Main", "");
    shell.fixture.refresh(&root);
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    assert!(!shell.present(row, cx), "the row went");
    assert!(has_sheet(&shell, cx));
    assert!(
        focused(&shell, "sheet-waiting-on", cx),
        "focus stays in the sheet, on the field that held the row"
    );
    shell.keys("escape", cx);
    assert!(!has_sheet(&shell, cx));
    assert!(focused(&shell, ChipField::All.id(), cx));
}

/// Opening, using and closing the sheet of a ticket and of a document
/// changes nothing in the repository and submits nothing.
#[gpui_kit::test]
fn the_sheet_writes_nothing(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, BLOCKER, "Dependency", "open");
        fixture.commit_ticket(
            root,
            MAIN,
            "Main",
            &format!("slug: {SLUG}\ndeps: [{BLOCKER}]\nzeta: last\n"),
        );
        fixture.commit_file(
            root,
            "docs/kept.md",
            document_source(DOCUMENT, "Kept", "status: draft\n").as_bytes(),
        );
    });
    let before = RepositorySnapshot::take(&root);
    let requests = shell.requests(cx, |_| true).len();

    open(&shell, ticket(&root, MAIN), cx);
    shell.click(ChipField::All.id(), cx);
    shell.click("copy-slug", cx);
    shell.click("copy-id", cx);
    shell.keys("tab tab tab x enter", cx);
    shell.click("sheet-done", cx);
    shell.click(ChipField::WaitingOn.id(), cx);
    shell.keys("escape", cx);

    open(&shell, document(&root, DOCUMENT), cx);
    shell.click(ChipField::Status.id(), cx);
    shell.keys("x backspace", cx);
    shell.click("sheet-done", cx);

    assert_eq!(RepositorySnapshot::take(&root), before);
    assert_eq!(shell.requests(cx, |_| true).len(), requests);
}

/// A conforming item read by its path is named by its kind, with its ID,
/// not called nonconforming; a nonconforming file still is.
#[gpui_kit::test]
fn an_inspect_tab_of_a_conforming_item_is_not_called_nonconforming(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Readable", "");
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let inspect = |path: &str| OpenTarget::Inspect {
        repo: key(&root),
        worktree: key(&root),
        path: path.to_owned(),
    };
    open(&shell, inspect(&ticket_path(MAIN)), cx);
    assert!(!shell.present("item-nonconforming", cx));
    assert_eq!(shell.label("item-kind", cx).as_deref(), Some("Ticket"));
    assert_eq!(shell.label("item-id", cx).as_deref(), Some(MAIN));
    assert_eq!(shell.label("item-path", cx), Some(ticket_path(MAIN)));

    open(&shell, inspect(MARKER), cx);
    assert_eq!(
        shell.label("item-nonconforming", cx).as_deref(),
        Some(NONCONFORMING_FILE)
    );
    assert!(!shell.present("item-kind", cx));
}
