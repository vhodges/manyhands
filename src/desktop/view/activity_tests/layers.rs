//! Activity's layers and its navigation: one layer at a time among the row
//! menu, ⋯ and the repository menus (R-13), and `navigation_sends_nothing`
//! (P31, Activity's part).

use gpui_kit::{Point, TestAppContext};
use manyhands::results::ResultCode;

use super::{
    cases::{answered, fabricate},
    list_support::*,
    pending_cases::{PENDING_ROOT, listing, read_pending},
    toolbar::{more_open, press_filter},
};
use crate::desktop::{
    model::{activity::rows::RowKey, layout::PanelKind},
    services::requests::Intent,
    test_support::fixtures::Fixture,
    view::{
        activity_support::rows as log_rows,
        footer::menu::Placement,
        harness::{Shell, key},
        polling_window::PollingWindow,
        repositories::MenuAnchor,
        workspace::{ToolbarButton, Workspace},
    },
};

/// The layers a window can have open, one of which may be at a time (R-13).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layer {
    /// A row's menu.
    Row,
    /// Activity's ⋯.
    More,
    /// The repositories' More menu.
    RepositoriesMore,
    /// A repository row's menu.
    Repository,
    /// The dock menu of a panel's footer toggle.
    Dock,
}

const LAYERS: [Layer; 5] = [
    Layer::Row,
    Layer::More,
    Layer::RepositoriesMore,
    Layer::Repository,
    Layer::Dock,
];

/// The layers the Workspace holds open now.
fn open_layers(workspace: &Workspace) -> Vec<Layer> {
    [
        (workspace.activity_menu_open(), Layer::Row),
        (workspace.activity_more_open(), Layer::More),
        (workspace.repository_menu_open(), Layer::Repository),
        (workspace.dock_menu_open(), Layer::Dock),
    ]
    .into_iter()
    .filter_map(|(open, layer)| open.then_some(layer))
    .collect()
}

/// Opens `layer` in one update, as its press, key or click does, and reads
/// the layers in the same update, before the Kit's own dismissals (a menu
/// closing when another takes focus) can run: the Workspace's own bookkeeping
/// is what R-13 asks for. The repository menus are told apart by the button
/// or row that opened them.
fn open(
    layer: Layer,
    row: &RowKey,
    repository: &str,
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Vec<Layer> {
    let at = Point::default();
    let layers = shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            match layer {
                Layer::Row => workspace.open_activity_menu_at(row.clone(), at, window, cx),
                Layer::More => workspace.press_toolbar(ToolbarButton::ActivityMore, window, cx),
                Layer::RepositoriesMore => {
                    workspace.press_toolbar(ToolbarButton::RepositoriesMore, window, cx)
                }
                Layer::Repository => workspace.open_repository_menu(
                    repository.to_owned(),
                    MenuAnchor::Pointer(at),
                    window,
                    cx,
                ),
                Layer::Dock => workspace.open_dock_menu(
                    PanelKind::Activity,
                    Placement::Pointer(at),
                    window,
                    cx,
                ),
            }
            let mut open = open_layers(workspace);
            // The two repository menus share a field: say which opened.
            if layer == Layer::RepositoriesMore && open == [Layer::Repository] {
                open = vec![Layer::RepositoriesMore];
            }
            open
        })
    });
    shell.run(cx);
    layers
}

/// R-13: one layer at a time among the row menu, ⋯, the repository menus and
/// the dock menu, in every order: opening any of them leaves it alone open.
#[gpui_kit::test]
fn one_layer_at_a_time_in_activity(cx: &mut TestAppContext) {
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let (row, repository) = (rows.pending[0].clone(), key(&rows.root));
    let none = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, _| open_layers(workspace))
    };
    assert_eq!(none(shell, cx), [], "nothing is open at first");
    for first in LAYERS {
        for second in LAYERS.into_iter().filter(|layer| *layer != first) {
            assert_eq!(
                open(first, &row, &repository, shell, cx),
                [first],
                "{first:?} opens"
            );
            assert_eq!(
                open(second, &row, &repository, shell, cx),
                [second],
                "{second:?} after {first:?} leaves it alone open"
            );
        }
    }

    // Opened by the pointer, ⋯ holds focus, as the Kit's menus do: Escape
    // closes it.
    open(Layer::Row, &row, &repository, shell, cx);
    shell.click(ToolbarButton::ActivityMore.id(), cx);
    assert_eq!(none(shell, cx), [Layer::More]);
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
