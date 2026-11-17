//! Unreadable files in both lists: listed with their reason, never opened
//! (Part I). Split from `tickets_tests.rs` unchanged.

use gpui_kit::TestAppContext;

use super::{ALPHA, panel_count, until_settled};
use crate::desktop::{
    model::navigation::Pane, test_support::fixtures::Fixture, view::harness::Shell,
};

const UNREADABLE_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const FINE_DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";

/// P1, P28, R-38: a ticket file and a document file the library cannot read
/// are listed in their lists with their reason, once a refresh has met them,
/// and they do not open. The lists read the status once per load. A held
/// status read leaves them as they were, and so does one that stays busy
/// (Part I). The ticket is a raw directory with no item worktree, since
/// discovery drops a primary copy's problems inside a ticket directory that
/// has one (`L/repository/discovery.rs:802-804`).
#[gpui_kit::test]
async fn an_unreadable_file_is_listed_with_its_reason(cx: &mut TestAppContext) {
    use std::time::Duration;

    use manyhands::repository::{LeaseKind, RepositoryService};

    use crate::desktop::{
        model::{ticket_view::CANT_BE_READ_PILL, unreadable::UnreadableFile},
        services::{ReadKind, requests::Intent},
    };

    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, ALPHA, "Alpha", "");
    fixture.create_document(&root, FINE_DOCUMENT, "docs/fine.md", "Fine");
    fixture.refresh(&root);
    // Written after the refresh, so the index does not know them yet.
    let ticket_dir = root.join(".manyhands/tickets").join(UNREADABLE_TICKET);
    std::fs::create_dir_all(&ticket_dir).unwrap();
    std::fs::write(ticket_dir.join("ticket.md"), [0xff, 0xfe, 0x00]).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    fixture.write_non_utf8(&root, "docs/broken.md");
    let ticket_path = format!(".manyhands/tickets/{UNREADABLE_TICKET}/ticket.md");

    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.use_list(Pane::Tickets, cx);
    assert_eq!(shell.rows("ticket-row", cx), ["Alpha"]);
    assert_eq!(shell.rows("doc-node", cx), ["Fine"]);
    // One status read with the lists' load (P1).
    let status_reads = |shell: &Shell| shell.backend.reads_seen(ReadKind::IndexStatus);
    assert_eq!(status_reads(&shell), 1);
    let list_reads = shell.backend.reads_seen(ReadKind::Tickets);

    // A refresh meets the files; the lists reload with one more status read.
    // It is held, so the lists land first, as they may in any order: the
    // files come in alone, and each list shows them (P1).
    let held = shell.backend.hold_read(ReadKind::IndexStatus);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(refresh, cx).await;
    until_settled(&shell, cx, "the lists reloaded", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("1")
            && shell.backend.reads_seen(ReadKind::IndexStatus) == 2
    })
    .await;
    assert_eq!(shell.rows("ticket-row", cx), ["Alpha"]);
    assert_eq!(shell.rows("doc-node", cx), ["Fine"]);
    let _ = held.send(());
    until_settled(&shell, cx, "the files listed", |shell, cx| {
        shell.rows("ticket-row", cx).len() == 2 && shell.rows("doc-node", cx).len() == 2
    })
    .await;
    assert_eq!(status_reads(&shell), 2, "one more with the lists' reload");
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), list_reads + 1);

    // Under every lifecycle the file is listed by its path, "can't be read".
    let unreadable_row = |shell: &Shell, cx: &mut TestAppContext| {
        let at = shell
            .rows("ticket-row", cx)
            .iter()
            .position(|label| *label == ticket_path)
            .expect("the file is listed by its path");
        (at, shell.label(("ticket-status", at), cx))
    };
    for (filter, rows) in [
        ("filter-open", 2),
        ("filter-closed", 1),
        ("filter-all", 2),
        ("filter-open", 2),
    ] {
        shell.click(filter, cx);
        assert_eq!(shell.rows("ticket-row", cx).len(), rows, "{filter}");
        let (_, pill) = unreadable_row(&shell, cx);
        assert_eq!(pill.as_deref(), Some(CANT_BE_READ_PILL), "{filter}");
    }
    let footnote = shell.label("tickets-footnote", cx).unwrap();
    assert!(footnote.ends_with(" \u{b7} 1 can't be read"), "{footnote}");
    assert_eq!(panel_count(&shell, cx).as_deref(), Some("2"));

    // What the files are: the library puts them in the primary worktree,
    // which is the repository's root, so both are primary. The raw ticket
    // directory is in the primary copy, not an item worktree.
    let root_text = root.to_str().unwrap().to_owned();
    let file = |path: &str| UnreadableFile {
        path: path.to_owned(),
        worktree: root_text.clone(),
        primary: true,
    };
    shell.workspace.read_with(cx, |workspace, _| {
        assert_eq!(
            workspace.tickets().unreadable(),
            [file(&ticket_path)],
            "the ticket file"
        );
        assert_eq!(
            workspace.documents().unreadable(),
            [file("docs/broken.md")],
            "the document file"
        );
    });

    // The document is listed by its file name, with its own mark.
    assert_eq!(shell.rows("doc-node", cx), ["broken.md", "Fine"]);
    assert!(shell.present(("doc-unreadable", 0usize), cx));
    assert!(!shell.present(("doc-unreadable", 1usize), cx));

    // A click selects each and Enter opens neither (R-38).
    let opened = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.opened(cx))
    };
    let (at, _) = unreadable_row(&shell, cx);
    shell.click(("ticket-row", at), cx);
    assert!(shell.selected(("ticket-row", at), cx));
    shell.keys("enter", cx);
    assert_eq!(opened(&shell, cx), None, "a ticket file");
    assert!(shell.rows("tab-title", cx).is_empty());
    shell.use_list(Pane::Documents, cx);
    shell.click(("doc-node", 0usize), cx);
    assert!(shell.selected(("doc-node", 0usize), cx));
    shell.keys("enter", cx);
    assert_eq!(opened(&shell, cx), None, "a document file");
    assert!(shell.rows("tab-title", cx).is_empty());

    // A reload whose status read is held lands the lists, and the files
    // stay listed as they were (P28).
    let rows = |shell: &Shell, cx: &mut TestAppContext| {
        (
            shell.rows("ticket-row", cx),
            shell.rows("doc-node", cx),
            shell.present(("doc-unreadable", 0usize), cx),
            shell
                .label("tickets-footnote", cx)
                .is_some_and(|text| text.ends_with(" \u{b7} 1 can't be read")),
        )
    };
    let before = rows(&shell, cx);
    assert_eq!(before.0.len(), 2);
    let held = shell.backend.hold_read(ReadKind::IndexStatus);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the lists landed", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("2")
    })
    .await;
    assert_eq!(status_reads(&shell), 3);
    assert_eq!(rows(&shell, cx), before, "held");

    // Released while the repository is busy, the status read stays busy; it
    // never fails the lists, and the files stay.
    let lease = RepositoryService::hold_lease_for_testing(
        &root,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    let _ = held.send(());
    for _ in 0..4 {
        cx.executor().advance_clock(Duration::from_secs(1));
        shell.run(cx);
    }
    drop(lease);
    assert!(!shell.present("tickets-alert", cx));
    assert!(!shell.present("documents-alert", cx));
    assert_eq!(rows(&shell, cx), before, "busy");

    // One that fails does the same. Only the status read can be made to
    // fail: it is held while the lists land, then goes to a service whose
    // registry does not know the repository.
    let held = shell.backend.hold_read(ReadKind::IndexStatus);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the lists landed", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("2")
    })
    .await;
    shell.backend.replace_service(
        RepositoryService::open_at(&shell.fixture.parent().join("other-registry")).unwrap(),
    );
    let _ = held.send(());
    shell.run(cx);
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    assert!(!shell.present("tickets-alert", cx));
    assert!(!shell.present("documents-alert", cx));
    assert_eq!(rows(&shell, cx), before, "failed");
}
