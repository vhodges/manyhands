//! What the Settings sheet's window tests share (Wave 04 Cycle 04): the
//! fixtures, the reads of the workspace and the sheet, and the two-draw
//! settle a focus design A6 names is read after (Ruling 2). Moved here
//! unchanged from `settings_sheet_tests.rs`.

use std::path::{Path, PathBuf};

use gpui_kit::{
    Context, Entity, FocusHandle, Pixels, TestAppContext, Window,
    component::{WindowExt, input::InputState},
    test::TestWindowExt,
};
use manyhands::{
    drafts::{DraftFields, DraftId},
    repository::ItemDtoKind,
};

use super::super::SheetOwnerKind;
use crate::desktop::{
    model::{
        layout::{Layout, PanelKind},
        navigation::OpenTarget,
        stamp::SettingsPage,
        tabs::TabKey,
    },
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::{
        delaying::JobGate,
        fixtures::{Fixture, item_id, ticket_path, ticket_source},
    },
    view::{
        harness::{Shell, key},
        item::ChipField,
        settings::{SettingsHost, Shown},
        workspace::Workspace,
    },
};

pub(super) const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW0";
pub(super) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW1";
pub(super) const SENTINEL: &str = "sentinel-passphrase-c04t1";
pub(super) const PASSPHRASE: [&str; 2] = ["key-generate-passphrase", "key-generate-confirm"];
/// A window short enough that the Repositories page runs below the Settings
/// sheet's body's fold: the sheet is as tall as the window.
pub(super) const SHORT: f32 = 400.;
/// Activity's height in that window, which leaves the item area room for a
/// long ticket's header and region.
pub(super) const SHORT_ACTIVITY: f32 = 100.;

/// One repository, `a`, built by `build` and refreshed, in an open window.
pub(super) fn shell(
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// `MAIN`, with a body far taller than a short window, and `OTHER`.
pub(super) fn long_and_other(fixture: &Fixture, root: &Path) {
    let body: String = (0..200)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect();
    let source = format!("{}{body}", ticket_source(MAIN, "Main", "project: p\n"));
    fixture.commit_file(root, &ticket_path(MAIN), source.as_bytes());
    fixture.commit_ticket(root, OTHER, "Other", "");
}

pub(super) fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

pub(super) fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

pub(super) fn opened(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

pub(super) fn owner(shell: &Shell, cx: &mut TestAppContext) -> Option<SheetOwnerKind> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.sheet_owner())
}

pub(super) fn has_sheet(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_sheet(cx))
}

pub(super) fn has_dialog(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_dialog(cx))
}

pub(super) fn shown(shell: &Shell, cx: &mut TestAppContext) -> Shown {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

pub(super) fn drafts_shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx))
}

pub(super) fn layout(shell: &Shell, cx: &mut TestAppContext) -> Layout {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.layout().clone())
}

pub(super) fn item_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.item_offset(cx))
}

pub(super) fn column_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.content_offset())
}

pub(super) fn settings_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_offset(cx))
}

/// Two draws, with what each starts run after it: a focus design A6 names
/// is read after them, so a `focus_lost` or a render-time rule that moved
/// it on the first has had its frame.
pub(super) fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

/// Whether the element `id`, or something in it, holds focus, after two
/// draws.
pub(super) fn holds(
    shell: &Shell,
    id: impl Into<gpui_kit::ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    settle(shell, cx);
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the focused element is inside the key context `context`, after
/// two draws.
pub(super) fn in_context(shell: &Shell, context: &str, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| {
        window
            .context_stack()
            .iter()
            .any(|entry| entry.contains(context))
    })
}

/// Whether `handle` holds focus, after two draws.
pub(super) fn holds_handle(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| handle.is_focused(window))
}

/// The owner is Settings and a sheet is active, frame after frame, with
/// `page` shown.
pub(super) fn kept(shell: &Shell, page: SettingsPage, cx: &mut TestAppContext) {
    for frame in 0..3 {
        assert_eq!(
            owner(shell, cx),
            Some(SheetOwnerKind::Settings),
            "frame {frame}: Settings owns the sheet"
        );
        assert!(has_sheet(shell, cx), "frame {frame}: the sheet is active");
        shell.run(cx);
    }
    assert_eq!(shown(shell, cx), Shown::Settings(page));
}

/// Settings is closed: the host shows no page, nothing owns a sheet and
/// none is active.
pub(super) fn closed(shell: &Shell, way: &str, cx: &mut TestAppContext) {
    assert_eq!(shown(shell, cx), Shown::Tabs, "{way}: the host is closed");
    assert_eq!(owner(shell, cx), None, "{way}: no owner");
    assert!(!has_sheet(shell, cx), "{way}: no sheet is active");
}

pub(super) fn host(shell: &Shell, cx: &mut TestAppContext) -> Entity<SettingsHost> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone())
}

/// Opens Settings by Ctrl+, and shows Sharing & keys (until Task 5, the
/// Keys page's own entry), then types the sentinel into both masked inputs.
/// Gives the inputs.
pub(super) fn open_on_keys(shell: &Shell, cx: &mut TestAppContext) -> [Entity<InputState>; 2] {
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 2usize), cx);
    assert!(shell.present("settings-keys", cx));
    let host = host(shell, cx);
    let inputs = shell.visual(cx).update(|window, cx| {
        let keys = host.update(cx, |host, cx| host.keys(window, cx));
        let inputs = PASSPHRASE.map(|id| keys.read(cx).input(id));
        for input in &inputs {
            input.update(cx, |input, cx| input.set_value(SENTINEL, window, cx));
        }
        inputs
    });
    shell.run(cx);
    assert!(!emptied(&inputs, cx), "the passphrase is typed");
    inputs
}

pub(super) fn emptied(inputs: &[Entity<InputState>; 2], cx: &mut TestAppContext) -> bool {
    inputs
        .iter()
        .all(|input| input.read_with(cx, |input, _| input.value().is_empty()))
}

pub(super) fn activity_list(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.activity_pane().list_focus().clone()
    })
}

pub(super) fn displayed(shell: &Shell, kind: PanelKind, cx: &mut TestAppContext) -> bool {
    layout(shell, cx).is_displayed(kind)
}

/// Clicks Edit and waits for the edit editor.
pub(super) async fn edit(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("item-edit", cx);
    shell
        .until_drawn(cx, "the edit editor", |shell, cx| {
            let pane = shell
                .workspace
                .read_with(cx, |workspace, _| workspace.item_pane())
                .expect("an item tab is shown");
            pane.read_with(cx, |pane, _| pane.edit_host().is_some())
        })
        .await;
}

/// The draft the tab of `root`'s `MAIN` holds.
pub(super) fn draft(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Option<DraftId> {
    let tab = TabKey::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: MAIN.to_owned(),
    };
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    drafts.read_with(cx, |registry, _| registry.attached(&tab))
}

/// Whether the draft `id` holds the title `title`.
pub(super) fn titled(shell: &Shell, id: DraftId, title: &str, cx: &mut TestAppContext) -> bool {
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    drafts.read_with(cx, |registry, _| {
        matches!(
            registry.session(id).map(|session| session.current()),
            Some(DraftFields::Ticket { title: held, .. }) if held == title
        )
    })
}

/// Opens the details sheet on the Title, types "x" at its end, and waits
/// for the draft that keeps it. Gives the draft.
pub(super) async fn edit_the_title(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> DraftId {
    shell.click(ChipField::All.id(), cx);
    assert_eq!(owner(shell, cx), Some(SheetOwnerKind::Details));
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    shell
        .until_drawn(cx, "the draft to begin", |shell, cx| {
            draft(shell, root, cx).is_some()
        })
        .await;
    let id = draft(shell, root, cx).expect("the draft began");
    assert!(titled(shell, id, "Mainx", cx), "the draft holds the edit");
    id
}

/// Presses Tab until Create repository, the Repositories page's last
/// control, holds focus.
pub(super) fn tab_to_create(shell: &Shell, cx: &mut TestAppContext) {
    let mut presses = 0;
    while !shell.frame(cx, |window, _| {
        window.try_find("create-submit").and_then(|e| e.focused()) == Some(true)
    }) {
        presses += 1;
        assert!(presses < 80, "Tab never reached Create repository");
        shell.keys("tab", cx);
    }
}

/// A refresh of `root` running on the worker and held there, so quitting
/// waits for it. Gives the gate and the request.
pub(super) fn hold_a_job(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> (JobGate, DesktopRequestId) {
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    let root = root.to_owned();
    let id = shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(Intent::IndexRefresh { root }, Lane::User, None, None)
    });
    held.wait_reached();
    (held, id)
}

/// Raises a way into Settings as its trigger calls it, inside an update of
/// the workspace with its window.
pub(super) fn route(
    shell: &Shell,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>),
) {
    let workspace = shell.workspace.clone();
    shell
        .visual(cx)
        .update(|window, cx| workspace.update(cx, |workspace, cx| f(workspace, window, cx)));
    shell.run(cx);
}
