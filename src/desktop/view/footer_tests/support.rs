//! What the footer's tests share: reading a control, the layout, a panel
//! behind another tab, a finished request, and the modals.

use std::path::PathBuf;

use gpui_kit::{
    App, TestAppContext, Window,
    component::{
        WindowExt,
        dock::{
            DockPlacement::{Bottom, Left, Right},
            InsertTarget,
        },
    },
    test::TestWindowExt,
};
use manyhands::{
    repository::{ItemDtoKind, RequestId},
    results::{Envelope, Outcome, ResultCode, Scope},
};

use crate::desktop::{
    model::{
        layout::{Layout, PanelKind},
        navigation::OpenTarget,
    },
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        footer::Control,
        harness::{Shell, key},
        item::{ChipField, OpenDetails},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";

/// A control as the screen reader reads it: its name and toggled state.
pub(super) fn read(
    shell: &Shell,
    control: Control,
    cx: &mut TestAppContext,
) -> Option<(String, Option<bool>)> {
    shell.frame(cx, |window, _| {
        window.try_find(control.id()).map(|element| {
            (
                element.label().unwrap_or_default().to_owned(),
                element.checked(),
            )
        })
    })
}

pub(super) fn layout(shell: &Shell, cx: &mut TestAppContext) -> Layout {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.layout().clone())
}

pub(super) fn saves(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.layout_saves())
}

pub(super) fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window.try_find(id).and_then(|element| element.focused()) == Some(true)
    })
}

pub(super) fn open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("popup-menu", cx)
}

/// The Kit's own edit for `kind` dropped into `with`'s tab group, at the end,
/// without selecting it: the only way to a panel that is on and behind
/// another tab.
pub(super) fn put_behind(shell: &Shell, kind: PanelKind, with: PanelKind, cx: &mut TestAppContext) {
    let area = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.dock_area().clone());
    let (id, node) = area.read_with(cx, |area, cx| {
        let find = |wanted: PanelKind| {
            [Left, Right, Bottom]
                .into_iter()
                .filter_map(|placement| area.layout(placement))
                .find_map(|tree| {
                    let id = tree.panels().find(|id| {
                        area.panel(*id)
                            .is_some_and(|panel| panel.panel_name(cx) == wanted.name())
                    })?;
                    Some((id, tree.find_panel_node(id)?))
                })
                .expect("the panel is in a dock")
        };
        (find(kind).0, find(with).1)
    });
    shell.frame(cx, |window, cx| {
        area.update(cx, |area, cx| {
            area.move_panel(
                id,
                InsertTarget::Tabs {
                    node,
                    ix: None,
                    activate: false,
                },
                window,
                cx,
            )
        })
    });
    shell.run(cx);
}

/// A request that finished with `outcome`, under a code that agrees with it.
pub(super) fn finish(shell: &Shell, id: u64, outcome: Outcome, cx: &mut TestAppContext) {
    let operations = shell.operations(cx);
    operations.update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(DesktopRequest {
            id: DesktopRequestId(id),
            request_id: RequestId::new(),
            intent: Intent::IndexRefresh {
                root: PathBuf::from("/fixture/a"),
            },
            confirmation: None,
            lane: Lane::User,
            attempt: 1,
            state: RequestState::Queued,
        }));
        operations.apply(WorkerEvent::Finished(
            DesktopRequestId(id),
            Box::new(Envelope::mutation(
                "index refresh",
                Scope::default(),
                outcome,
                match outcome {
                    Outcome::Success => ResultCode::Ok,
                    _ => ResultCode::Busy,
                },
            )),
        ));
        cx.notify();
    });
    shell.run(cx);
}
/// A window with one ticket open in a tab, whose chips open the details
/// sheet.
pub(super) fn shell_with_a_ticket(cx: &mut TestAppContext) -> Shell {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Ticket", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell
}

/// Opens a dialog, as the app's are (the overlay does not close it), or the
/// ticket's details sheet, as its chip asks.
pub(super) fn open_modal(shell: &Shell, sheet: bool, cx: &mut TestAppContext) {
    if sheet {
        let pane = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .expect("the ticket's tab is shown");
        pane.update(cx, |_, cx| cx.emit(OpenDetails(ChipField::All)));
    } else {
        shell.frame(cx, |window, cx| {
            window.open_dialog(cx, |dialog, _, _| {
                dialog.title("Probe").overlay_closable(false)
            })
        });
    }
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert!(modal_open(shell, cx), "the modal is open (sheet {sheet})");
}

pub(super) fn modal_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window: &mut Window, cx: &mut App| {
        window.has_active_dialog(cx) || window.has_active_sheet(cx)
    })
}
