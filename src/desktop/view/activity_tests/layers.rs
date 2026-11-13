//! Activity's layers and its navigation: one layer at a time among the row
//! menu, ⋯ and the repository menus (R-13), and `navigation_sends_nothing`
//! (P31, Activity's part).

use gpui_kit::TestAppContext;
use manyhands::results::ResultCode;

use super::{
    cases::{answered, fabricate},
    list_support::*,
    pending_cases::{PENDING_ROOT, listing, read_pending},
    toolbar::{more_open, press_filter},
};
use crate::desktop::{
    model::activity::rows::RowKey,
    services::requests::Intent,
    test_support::fixtures::Fixture,
    view::{
        activity_support::rows as log_rows, harness::Shell, polling_window::PollingWindow,
        workspace::ToolbarButton,
    },
};

fn layers(shell: &Shell, cx: &mut TestAppContext) -> (bool, bool, bool) {
    shell.workspace.read_with(cx, |workspace, _| {
        (
            workspace.activity_menu_open(),
            workspace.activity_more_open(),
            workspace.repository_menu_open(),
        )
    })
}

/// Runs `f` on the Workspace and reads the layers in the same update, before
/// the Kit's own dismissals (a menu closing when another takes focus) can
/// run: the Workspace's own bookkeeping is what R-13 asks for.
fn then_layers(
    shell: &Shell,
    cx: &mut TestAppContext,
    f: impl FnOnce(
        &mut crate::desktop::view::workspace::Workspace,
        &mut gpui_kit::Window,
        &mut gpui_kit::Context<crate::desktop::view::workspace::Workspace>,
    ),
) -> (bool, bool, bool) {
    let layers = shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            f(workspace, window, cx);
            (
                workspace.activity_menu_open(),
                workspace.activity_more_open(),
                workspace.repository_menu_open(),
            )
        })
    });
    shell.run(cx);
    layers
}

/// R-13: one layer at a time among the row menu, ⋯ and the repository menus,
/// in each direction. Each opens through the same entry point its press or
/// key calls, and the Workspace's own state is read before anything draws, so
/// a missing close shows even though the Kit's menus dismiss themselves when
/// another takes focus.
#[gpui_kit::test]
fn one_layer_at_a_time_in_activity(cx: &mut TestAppContext) {
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let key = rows.pending[0].clone();
    let open_row_menu = |key: RowKey| {
        move |workspace: &mut crate::desktop::view::workspace::Workspace,
              window: &mut gpui_kit::Window,
              cx: &mut gpui_kit::Context<crate::desktop::view::workspace::Workspace>| {
            workspace.open_activity_menu_at(key.clone(), gpui_kit::Point::default(), window, cx)
        }
    };
    let press = |button: ToolbarButton| {
        move |workspace: &mut crate::desktop::view::workspace::Workspace,
              window: &mut gpui_kit::Window,
              cx: &mut gpui_kit::Context<crate::desktop::view::workspace::Workspace>| {
            workspace.press_toolbar(button, window, cx)
        }
    };

    assert_eq!(layers(shell, cx), (false, false, false));
    // The row menu, then ⋯: the row menu is closed.
    assert_eq!(
        then_layers(shell, cx, open_row_menu(key.clone())),
        (true, false, false)
    );
    assert_eq!(
        then_layers(shell, cx, press(ToolbarButton::ActivityMore)),
        (false, true, false),
        "⋯ closed the row menu"
    );
    // ⋯, then the row menu: ⋯ is closed.
    assert_eq!(
        then_layers(shell, cx, open_row_menu(key.clone())),
        (true, false, false),
        "the row menu closed ⋯"
    );
    // The repository More menu, then ⋯, then it again, in both directions.
    assert_eq!(
        then_layers(shell, cx, press(ToolbarButton::RepositoriesMore)),
        (false, false, true),
        "the repository menu closed the row menu"
    );
    assert_eq!(
        then_layers(shell, cx, press(ToolbarButton::ActivityMore)),
        (false, true, false),
        "⋯ closed the repository menu"
    );
    assert_eq!(
        then_layers(shell, cx, press(ToolbarButton::RepositoriesMore)),
        (false, false, true),
        "the repository menu closed ⋯"
    );
    // Opened by the pointer, ⋯ holds focus, as the Kit's menus do: a choice
    // reaches its handler with the row menu closed.
    then_layers(shell, cx, open_row_menu(key));
    shell.click(ToolbarButton::ActivityMore.id(), cx);
    assert_eq!(layers(shell, cx), (false, true, false));
    shell.keys("escape", cx);
    assert!(!more_open(shell, cx), "Escape closes ⋯");
}

/// P31, Activity's part: moving the highlight, toggling the filter, opening
/// and closing the row menu and ⋯ send nothing, over the recording backend:
/// no request is made and no poll is called, and the log is as it was.
#[gpui_kit::test]
fn navigation_sends_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let (window, ()) = PollingWindow::over(fixture, std::slice::from_ref(&root), |_, _| (), cx);
    let shell = &window.shell;
    fabricate(
        shell,
        1,
        Intent::IndexRefresh { root },
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    read_pending(
        shell,
        PENDING_ROOT,
        listing(vec![save(OP_A), save(OP_B)]),
        cx,
    );
    settle(shell, cx);
    let footprint = |cx: &mut TestAppContext| {
        (
            shell.requests(cx, |_| true).len(),
            window.backend.calls().len(),
            log_rows(shell, cx),
            pressed(shell, cx),
        )
    };
    let before = footprint(cx);
    assert!(before.2.len() >= 3, "rows to move over: {:?}", before.2);

    // The highlight: into the list, over every row, to both ends.
    shell.keys("secondary-4", cx);
    for key in ["down", "down", "up", "end", "home"] {
        shell.keys(key, cx);
    }
    // The row menu, by key and by pointer, closed by Escape.
    shell.keys("shift-f10", cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.activity_menu_open()),
        "the row menu opened"
    );
    shell.keys("escape", cx);
    // The filter, on and off, and ⋯ opened and closed.
    press_filter(shell, cx);
    press_filter(shell, cx);
    shell.click(ToolbarButton::ActivityMore.id(), cx);
    assert!(more_open(shell, cx), "⋯ opened");
    shell.keys("escape", cx);
    assert!(!more_open(shell, cx));
    assert_eq!(footprint(cx), before, "nothing was sent, nothing changed");
}
