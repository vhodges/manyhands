//! A hidden panel loses nothing: its state is kept, a request finishes and
//! shows its entry when Activity does, and Show it reveals Activity (P2,
//! P28, R-10).

use gpui_kit::TestAppContext;

use super::support::*;
use crate::desktop::{
    model::{
        activity::rows::{Kind, RowKey},
        layout::PanelKind,
        navigation::{OpenTarget, Pane},
    },
    services::{
        JobKind,
        requests::{Intent, RequestState},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{highlighted, list_focused, refresh, row},
        harness::{Shell, key},
        item::ItemRoute,
        workspace::Region,
    },
};
use PanelKind::{Activity, Tickets};
use manyhands::repository::ItemDtoKind;

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";

/// P2, P26: Tickets, with a filter, filter text, a sort, a readiness and a
/// selection, is hidden and shown again, and has them all.
#[gpui_kit::test]
fn a_hidden_panel_keeps_its_state(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, FIRST, "First", "open");
    fixture.create_ticket(&root, SECOND, "Second", "open");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    shell.use_list(Pane::Tickets, cx);
    shell.click("filter-all", cx);
    // Filter text that keeps both rows, a sort and a readiness.
    act(&shell, cx, |workspace, window, cx| {
        workspace.set_filter_text("s", window, cx)
    });
    shell.click("sort-title", cx);
    shell.click("filter-readiness", cx);
    shell.keys("down down enter", cx);
    shell.keys("secondary-2 down down", cx);
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    let state = |shell: &Shell, cx: &mut TestAppContext| {
        (
            shell.selected("filter-all", cx),
            input.read_with(cx, |input, _| input.value().to_string()),
            shell.label("filter-readiness", cx),
            shell.label("sort-title", cx),
            shell.workspace.read_with(cx, |workspace, cx| {
                workspace.tickets_pane().selected_row(cx)
            }),
            shell.rows("ticket-row", cx),
        )
    };
    let before = state(&shell, cx);
    assert!(
        before.0 && before.4.is_some() && before.5.len() == 2,
        "setup: {before:?}"
    );
    assert_eq!(before.1, "s");
    assert_eq!(before.2.as_deref(), Some("Readiness: ready"));
    assert_eq!(before.3.as_deref(), Some("Sort by title, A to Z"));
    // The model keeps them too, not only the elements.
    let query = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tickets().query().clone());
    assert_eq!(query.text, "s");
    assert!(query.readiness.is_some());

    toggle_panel(&shell, Tickets, cx);
    assert!(!shell.present(Region::Tickets.id(), cx));
    shell.frame(cx, |_, _| ());
    toggle_panel(&shell, Tickets, cx);

    assert_eq!(state(&shell, cx), before);
}

/// P28 and the carried check that nothing is pruned while Activity is
/// hidden: a request finishes with Activity hidden and moved, and its entry
/// is there when Activity is shown. Hiding and moving open, close and
/// reload no tab, and send the worker nothing.
#[gpui_kit::test]
async fn a_request_finishes_while_activity_is_hidden(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, FIRST, "First", "");
    fixture.refresh(&root);
    let mut gate = None;
    let shell = Shell::launch(
        fixture,
        |backend| gate = Some(backend.hold_job(JobKind::IndexRefresh)),
        cx,
    );
    let gate = gate.unwrap();
    gate.wait_reached();
    shell.run(cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(FIRST),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    let entry = RowKey::Request(id);
    refresh(&shell, cx);
    let running = row(&shell, &entry, cx).expect("its entry is listed while it runs");
    assert_eq!(running.row.kind, Kind::Working);
    let (jobs, tabs) = (shell.backend.jobs_seen(), shell.rows("tab-title", cx));
    assert_eq!(tabs.len(), 1, "setup: a tab is open");

    dock_panel(
        &shell,
        Activity,
        crate::desktop::model::layout::Side::Left,
        cx,
    );
    toggle_panel(&shell, Activity, cx);
    assert!(!shell.present(Region::Operations.id(), cx));
    assert_eq!(shell.backend.jobs_seen(), jobs, "nothing was sent");
    assert_eq!(shell.rows("tab-title", cx), tabs);

    gate.release();
    shell.until_ended(id, cx).await;
    shell.run(cx);
    assert!(!layout(&shell, cx).is_displayed(Activity), "hidden");
    refresh(&shell, cx);
    let finished = row(&shell, &entry, cx).expect("hidden, but its entry is not pruned");
    assert_eq!(finished.row.kind, Kind::Success);
    assert!(matches!(
        shell
            .operations(cx)
            .read_with(cx, |operations, _| operations
                .request(id)
                .map(|request| request.state.clone())),
        Some(RequestState::Finished(..))
    ));
    let (reads, jobs) = (shell.backend.reads_in_order(), shell.backend.jobs_seen());

    toggle_panel(&shell, Activity, cx);
    assert!(layout(&shell, cx).is_displayed(Activity), "shown");
    assert!(shell.present(Region::Operations.id(), cx), "drawn");
    refresh(&shell, cx);
    assert_eq!(
        row(&shell, &entry, cx).expect("its entry is listed with Activity"),
        finished,
        "the entry is as it was when hidden"
    );
    assert_eq!(
        shell.rows("tab-title", cx),
        tabs,
        "no tab opened, closed or reloaded"
    );
    assert_eq!(
        shell.backend.reads_in_order(),
        reads,
        "showing it read nothing"
    );
    assert_eq!(shell.backend.jobs_seen(), jobs, "showing it sent nothing");
}

/// R-10: Show it reveals Activity before it highlights the row of the
/// request that may hold the repository and focuses the list, or, with none,
/// focuses the region. The row is in the log while Activity is hidden.
#[gpui_kit::test]
async fn show_it_reveals_a_hidden_activity(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.refresh(&root);
    let shell = Shell::launch(fixture, |_| {}, cx);
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(id, cx).await;
    shell.frame(cx, |_, _| ());
    refresh(&shell, cx);
    let holder = RowKey::Request(id);
    assert!(
        row(&shell, &holder, cx).is_some(),
        "setup: the request has its row"
    );

    for (what, request) in [("its entry", Some(id)), ("the region", None)] {
        toggle_panel(&shell, Activity, cx);
        assert!(
            !shell.present(Region::Operations.id(), cx),
            "{what}: hidden"
        );
        shell.frame(cx, |_, _| ());
        let route = ItemRoute::Holder {
            root: root.clone(),
            request,
        };
        shell.frame(cx, |window, cx| {
            shell
                .workspace
                .update(cx, |workspace, cx| workspace.open_route(route, window, cx))
        });
        shell.run(cx);
        shell.frame(cx, |_, _| ());

        assert!(
            layout(&shell, cx).is_displayed(Activity),
            "{what}: revealed"
        );
        assert!(shell.present(Region::Operations.id(), cx), "{what}: drawn");
        assert_eq!(
            focus(&shell, cx),
            Some(Region::Operations),
            "{what}: focused"
        );
        assert!(list_focused(&shell, cx), "{what}: the list has focus");
        if request.is_some() {
            assert_eq!(
                highlighted(&shell, cx),
                Some(holder.clone()),
                "{what}: its row is highlighted"
            );
        }
        assert_agreement(&shell, what, cx);
    }
}
