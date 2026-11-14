//! Settings as a sheet the workspace keeps (Wave 04 Cycle 04 design Part
//! A, B2 and C3), over real repositories: the workspace keeps the sheet
//! whatever the item area shows (P1); the item, its scroll and its draft are
//! as they were once it closes (P2); it replaces the details sheet, whose
//! edits the draft keeps (P3); it closes every way, with a typed passphrase
//! emptied (P6); nothing acts behind it (P7); it scrolls on its own (P8);
//! Show in Activity closes it and focuses Activity (P14); and no route opens
//! it under a dialog or while quitting (P15).
//!
//! A focus that design A6 names is read after two draws (Ruling 2). Guards
//! are tested with dispatched actions, which the scrim does not take. A
//! test that waits on the worker allows parking. Draft fields and typed
//! secrets would print through `Debug`, so they are compared with
//! `assert!`.

use std::path::{Path, PathBuf};

use gpui_kit::{
    Action, Context, Entity, FocusHandle, Modifiers, Pixels, TestAppContext, Window,
    component::{WindowExt, input::InputState},
    point, px,
    test::TestWindowExt,
};
use manyhands::{
    drafts::{DraftFields, DraftId},
    repository::ItemDtoKind,
};

use super::SheetOwnerKind;
use crate::desktop::{
    keys::{
        CloseTab, FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NewItem,
        NextRegion, NextTab, OpenDrafts, OpenSettings, PreviousRegion, PreviousTab,
        ToggleBottomDock, ToggleLeftDock, ToggleRightDock,
    },
    model::{
        layout::{Layout, PanelKind},
        navigation::OpenTarget,
        polling::Answer,
        stamp::{SettingsPage, TabId},
        tabs::TabKey,
    },
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::{
        delaying::JobGate,
        fixtures::{Fixture, item_id, ticket_path, ticket_source},
    },
    view::{
        footer::Control,
        harness::{Shell, key},
        item::ChipField,
        settings::{SettingsHost, Shown, polling_support::Rig},
        workspace::{QuitPhase, Region},
    },
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW1";
const SENTINEL: &str = "sentinel-passphrase-c04t1";
const PASSPHRASE: [&str; 2] = ["key-generate-passphrase", "key-generate-confirm"];
/// A window short enough that the Repositories page runs below the Settings
/// sheet's body's fold: the sheet is as tall as the window.
const SHORT: f32 = 400.;
/// Activity's height in that window, which leaves the item area room for a
/// long ticket's header and region.
const SHORT_ACTIVITY: f32 = 100.;

/// One repository, `a`, built by `build` and refreshed, in an open window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// `MAIN`, with a body far taller than a short window, and `OTHER`.
fn long_and_other(fixture: &Fixture, root: &Path) {
    let body: String = (0..200)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect();
    let source = format!("{}{body}", ticket_source(MAIN, "Main", "project: p\n"));
    fixture.commit_file(root, &ticket_path(MAIN), source.as_bytes());
    fixture.commit_ticket(root, OTHER, "Other", "");
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

fn owner(shell: &Shell, cx: &mut TestAppContext) -> Option<SheetOwnerKind> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.sheet_owner())
}

fn has_sheet(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_sheet(cx))
}

fn has_dialog(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_dialog(cx))
}

fn shown(shell: &Shell, cx: &mut TestAppContext) -> Shown {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

fn drafts_shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx))
}

fn layout(shell: &Shell, cx: &mut TestAppContext) -> Layout {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.layout().clone())
}

fn item_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.item_offset(cx))
}

fn column_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.content_offset())
}

fn settings_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_offset(cx))
}

/// Two draws, with what each starts run after it: a focus design A6 names
/// is read after them, so a `focus_lost` or a render-time rule that moved
/// it on the first has had its frame.
fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

/// Whether the element `id`, or something in it, holds focus, after two
/// draws.
fn holds(shell: &Shell, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) -> bool {
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
fn in_context(shell: &Shell, context: &str, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| {
        window
            .context_stack()
            .iter()
            .any(|entry| entry.contains(context))
    })
}

/// Whether `handle` holds focus, after two draws.
fn holds_handle(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| handle.is_focused(window))
}

/// The owner is Settings and a sheet is active, frame after frame, with
/// `page` shown.
fn kept(shell: &Shell, page: SettingsPage, cx: &mut TestAppContext) {
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
fn closed(shell: &Shell, way: &str, cx: &mut TestAppContext) {
    assert_eq!(shown(shell, cx), Shown::Tabs, "{way}: the host is closed");
    assert_eq!(owner(shell, cx), None, "{way}: no owner");
    assert!(!has_sheet(shell, cx), "{way}: no sheet is active");
}

fn host(shell: &Shell, cx: &mut TestAppContext) -> Entity<SettingsHost> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone())
}

/// Opens Settings by Ctrl+, and shows Sharing & keys (until Task 5, the
/// Keys page's own entry), then types the sentinel into both masked inputs.
/// Gives the inputs.
fn open_on_keys(shell: &Shell, cx: &mut TestAppContext) -> [Entity<InputState>; 2] {
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

fn emptied(inputs: &[Entity<InputState>; 2], cx: &mut TestAppContext) -> bool {
    inputs
        .iter()
        .all(|input| input.read_with(cx, |input, _| input.value().is_empty()))
}

fn activity_list(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.activity_pane().list_focus().clone()
    })
}

fn displayed(shell: &Shell, kind: PanelKind, cx: &mut TestAppContext) -> bool {
    layout(shell, cx).is_displayed(kind)
}

/// Clicks Edit and waits for the edit editor.
async fn edit(shell: &Shell, cx: &mut TestAppContext) {
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
fn draft(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Option<DraftId> {
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
fn titled(shell: &Shell, id: DraftId, title: &str, cx: &mut TestAppContext) -> bool {
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
async fn edit_the_title(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> DraftId {
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
fn tab_to_create(shell: &Shell, cx: &mut TestAppContext) {
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
fn hold_a_job(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> (JobGate, DesktopRequestId) {
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
fn route(
    shell: &Shell,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut super::Workspace, &mut Window, &mut Context<super::Workspace>),
) {
    let workspace = shell.workspace.clone();
    shell
        .visual(cx)
        .update(|window, cx| workspace.update(cx, |workspace, cx| f(workspace, window, cx)));
    shell.run(cx);
}

/// P1 (A1, R-2, R-31, R-33, R-51): after Ctrl+, the owner is Settings and a
/// sheet is active frame after frame: with an item shown, with Drafts shown
/// under it, and after `open_item` lands, as the filter's ID read does. A
/// details sheet asked for in the same frame as Ctrl+, never opens.
#[gpui_kit::test]
fn settings_is_a_sheet_the_workspace_keeps(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
        fixture.commit_ticket(root, OTHER, "Other", "");
    });
    open(&shell, ticket(&root, MAIN), cx);

    // With an item shown, which stays shown under the sheet.
    shell.keys("secondary-,", cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, MAIN)));
    // An item opened by a landing leaves Settings open, the item under it.
    open(&shell, ticket(&root, OTHER), cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, OTHER)));
    shell.keys("escape", cx);
    closed(&shell, "Escape over an item", cx);

    // With Drafts shown, which stays shown under the sheet and after it.
    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    shell.keys("secondary-,", cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert!(
        drafts_shown(&shell, cx),
        "Drafts stays shown under the sheet"
    );
    shell.keys("escape", cx);
    closed(&shell, "Escape over Drafts", cx);
    assert!(
        drafts_shown(&shell, cx),
        "Drafts is shown once Settings closes"
    );

    // A details sheet asked for in the same frame as Ctrl+, is dropped.
    open(&shell, ticket(&root, MAIN), cx);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown");
    let workspace = shell.workspace.clone();
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.ask_details(&pane, ChipField::Project, cx)
        });
        window.dispatch_action(OpenSettings.boxed_clone(), cx);
    });
    shell.run(cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert!(
        !shell.present("sheet-heading", cx),
        "no details sheet opened"
    );
    shell.keys("escape", cx);
    closed(&shell, "Escape", cx);
    assert!(
        !shell.present("sheet-heading", cx),
        "nor does one once Settings closes"
    );
}

/// P2 (contract 2; the Wave's exit evidence): with an item open, its region
/// scrolled and a field edited into its draft, Ctrl+, then Tab to a control
/// below the fold, then Escape, leave the same tab shown, the tabs as they
/// were, the item's offset as it was, and the draft holding the edit. (Its
/// `AddRepository` row is Task 6's.)
#[gpui_kit::test]
async fn settings_leaves_the_item_as_it_was(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, long_and_other);
    shell.resize_keeping_content(1440., SHORT, cx);
    shell.set_activity_height(SHORT_ACTIVITY, cx);
    open(&shell, ticket(&root, OTHER), cx);
    open(&shell, ticket(&root, MAIN), cx);
    edit(&shell, cx).await;
    let draft = edit_the_title(&shell, &root, cx).await;
    shell.keys("escape", cx);
    assert!(!has_sheet(&shell, cx), "the details sheet closed");
    shell.keys("secondary-3", cx);
    shell.keys("pagedown", cx);
    let offset = item_offset(&shell, cx);
    assert!(offset < px(0.), "the item's region scrolled: {offset:?}");
    let (tab, tabs): (Option<TabId>, Vec<TabKey>) =
        shell.workspace.read_with(cx, |workspace, _| {
            (workspace.shown_tab(), workspace.tab_keys())
        });

    shell.keys("secondary-,", cx);
    kept(&shell, SettingsPage::Repositories, cx);
    tab_to_create(&shell, cx);
    assert!(
        settings_offset(&shell, cx) < px(0.),
        "Settings scrolled to the control"
    );
    assert_eq!(item_offset(&shell, cx), offset, "the item did not move");
    shell.keys("escape", cx);
    closed(&shell, "Escape", cx);

    let (now_tab, now_tabs) = shell.workspace.read_with(cx, |workspace, _| {
        (workspace.shown_tab(), workspace.tab_keys())
    });
    assert_eq!(now_tab, tab, "the same tab is shown");
    assert_eq!(now_tabs, tabs, "the tabs are as they were");
    assert_eq!(item_offset(&shell, cx), offset, "the item kept its place");
    assert!(
        titled(&shell, draft, "Mainx", cx),
        "the draft holds the edit"
    );
}

/// P3 (contract 3; Wave decision 6; R-6, R-8; A6 F3): with the details sheet
/// open and a field edited, Ctrl+, opens Settings in its place; the details
/// sheet is gone and the draft holds the edit. Focus is on the page's
/// entry, and Escape returns it to the chip that opened the details sheet.
#[gpui_kit::test]
async fn settings_replaces_the_details_sheet_and_keeps_its_edits(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
    });
    open(&shell, ticket(&root, MAIN), cx);
    edit(&shell, cx).await;
    let draft = edit_the_title(&shell, &root, cx).await;

    shell.keys("secondary-,", cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert_eq!(owner(&shell, cx), Some(SheetOwnerKind::Settings));
    assert!(has_sheet(&shell, cx));
    assert!(
        !shell.present("sheet-heading", cx),
        "the details sheet is gone"
    );
    assert!(
        titled(&shell, draft, "Mainx", cx),
        "the draft holds the edit"
    );
    assert!(
        holds(&shell, ("settings-page", 0usize), cx),
        "F3: the page's entry"
    );

    shell.keys("escape", cx);
    closed(&shell, "Escape", cx);
    assert!(
        holds(&shell, ChipField::All.id(), cx),
        "F3: on close, the chip"
    );
    assert!(
        titled(&shell, draft, "Mainx", cx),
        "the draft still holds it"
    );
}

/// How P6 closes Settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Way {
    Escape,
    CloseButton,
    Scrim,
    QuitWithActivity,
    QuitWithoutActivity,
    QuitOverConfirmation,
}

/// Closes the open Settings `way`, for the ways that do not quit.
fn close(shell: &Shell, way: Way, cx: &mut TestAppContext) {
    match way {
        Way::Escape => shell.keys("escape", cx),
        Way::CloseButton => {
            shell.frame(cx, |window, cx| {
                window.within("sheet-content").click("close", cx)
            });
            shell.run(cx);
        }
        Way::Scrim => {
            // Left of the 720 px sheet, on its overlay.
            let mut visual = shell.visual(cx);
            visual.simulate_click(point(px(120.), px(600.)), Modifiers::default());
            visual.run_until_parked();
        }
        _ => unreachable!("{way:?} quits"),
    }
}

/// P6 (A3, R-10, R-28, R-29, R-52; A6 F11 to F14): Escape, the close button,
/// the scrim, Show in Activity, quitting with Activity displayed, quitting
/// with Activity hidden, quitting with a confirmation open inside Settings
/// and quitting with unkept edits each leave `Shown::Tabs`, no owner, no
/// active sheet, the host closed and a typed passphrase empty. Show in
/// Activity focuses the Activity list (F11); quitting focuses Quit now while
/// Activity is displayed (F12), else the origin (F13); with unkept edits the
/// EditsNotKept dialog holds focus, and its Cancel focuses the content
/// region as before (F14).
#[gpui_kit::test]
async fn settings_closes_every_way(cx: &mut TestAppContext) {
    // A repository waiting for an unfinished operation: its row on Checking
    // for updates offers Show in Activity.
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.answer(
        0,
        Answer::Repository {
            recovery_required: true,
        },
        cx,
    );
    let shell = &rig.shell;
    for way in [Way::Escape, Way::CloseButton, Way::Scrim] {
        let inputs = open_on_keys(shell, cx);
        kept(shell, SettingsPage::Keys, cx);
        close(shell, way, cx);
        closed(shell, &format!("{way:?}"), cx);
        assert!(emptied(&inputs, cx), "{way:?}: the passphrase is emptied");
    }
    // Show in Activity (F11).
    let inputs = open_on_keys(shell, cx);
    shell.click(("settings-page", 4usize), cx);
    shell
        .until_drawn(cx, "the page's rows", |shell, cx| {
            shell.present(("polling-name", 0usize), cx)
        })
        .await;
    let row = rig.row("a", cx);
    shell.click(("polling-pending", row), cx);
    closed(shell, "Show in Activity", cx);
    assert!(emptied(&inputs, cx), "Show in Activity: the passphrase");
    let list = activity_list(shell, cx);
    assert!(holds_handle(shell, &list, cx), "F11: the Activity list");

    for way in [
        Way::QuitWithActivity,
        Way::QuitWithoutActivity,
        Way::QuitOverConfirmation,
    ] {
        let (shell, root) = self::shell(cx, |fixture, root| {
            fixture.commit_ticket(root, MAIN, "Main", "");
        });
        if way == Way::QuitWithoutActivity {
            shell.click(Control::Panel(PanelKind::Activity).id(), cx);
            assert!(!displayed(&shell, PanelKind::Activity, cx));
        }
        let (held, id) = hold_a_job(&shell, &root, cx);
        let origin = shell
            .frame(cx, |window, cx| window.focused(cx))
            .expect("something holds focus");
        let inputs = open_on_keys(&shell, cx);
        if way == Way::QuitOverConfirmation {
            shell.click(("settings-page", 0usize), cx);
            let rows = shell.rows("managed-row", cx);
            let ix = rows
                .iter()
                .position(|row| *row == key(&root))
                .expect("a is listed");
            shell.click(("managed-choose", ix), cx);
            shell.click("remove-submit", cx);
            assert!(has_dialog(&shell, cx), "the confirmation is open");
        }
        shell.keys("secondary-q", cx);
        let phase = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase());
        assert_eq!(phase, QuitPhase::Finishing(id), "{way:?}");
        closed(&shell, &format!("{way:?}"), cx);
        assert!(!has_dialog(&shell, cx), "{way:?}: no dialog is open");
        assert!(emptied(&inputs, cx), "{way:?}: the passphrase is emptied");
        if way == Way::QuitWithoutActivity {
            assert!(holds_handle(&shell, &origin, cx), "F13: the origin");
        } else {
            assert!(holds(&shell, "quit-now", cx), "F12: Quit now");
        }
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

    #[cfg(unix)]
    quit_with_unkept_edits(cx).await;
}

/// P6's last row (F14): a tab's edit that no draft holds, crash recovery
/// being off (the drafts folder is a link, so the store does not open).
#[cfg(unix)]
async fn quit_with_unkept_edits(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, MAIN, "Main", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    shell
        .until_drawn(cx, "the edit editor", |shell, cx| {
            let pane = shell
                .workspace
                .read_with(cx, |workspace, _| workspace.item_pane())
                .expect("an item tab is shown");
            pane.read_with(cx, |pane, _| pane.edit_host().is_some())
        })
        .await;
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    shell.keys("escape", cx);
    assert_eq!(draft(&shell, &root, cx), None, "no draft holds the edit");

    let inputs = open_on_keys(&shell, cx);
    shell.keys("secondary-q", cx);
    assert!(shell.present("edits-not-kept-dialog", cx), "quitting asks");
    closed(&shell, "QuitWithUnkeptEdits", cx);
    assert!(
        emptied(&inputs, cx),
        "unkept edits: the passphrase is emptied"
    );
    assert!(has_dialog(&shell, cx));
    assert!(
        in_context(&shell, "Dialog", cx),
        "F14: the EditsNotKept dialog"
    );
    shell.click("edits-not-kept-cancel", cx);
    assert!(!has_dialog(&shell, cx));
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Running, "Cancel stays");
    settle(&shell, cx);
    assert!(
        shell.present(Region::Content.ring_id(), cx),
        "F14: Cancel focuses the content region, as before"
    );
}

/// P7 (A5, R-60, R-8; contract 7): with Settings open on Sharing & keys, the
/// panel and region actions, `FocusContent`, `NewItem`, the tab actions and
/// `OpenDrafts`, dispatched where focus is, change nothing; `OpenSettings`
/// leaves the page as it was.
#[gpui_kit::test]
fn nothing_acts_behind_settings(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
        fixture.commit_ticket(root, OTHER, "Other", "");
    });
    shell.select(&root, cx);
    open(&shell, ticket(&root, OTHER), cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 2usize), cx);
    assert_eq!(shown(&shell, cx), Shown::Settings(SettingsPage::Keys));

    #[derive(PartialEq, Debug)]
    struct Footprint {
        shown: Shown,
        owner: Option<SheetOwnerKind>,
        sheet: bool,
        dialog: bool,
        layout: Layout,
        tabs: Vec<TabKey>,
        tab: Option<TabId>,
        drafts: bool,
        focus: Option<FocusHandle>,
        requests: usize,
    }
    let footprint = |cx: &mut TestAppContext| {
        let (focus, sheet, dialog) = shell.frame(cx, |window, cx| {
            (
                window.focused(cx),
                window.has_active_sheet(cx),
                window.has_active_dialog(cx),
            )
        });
        let (shown, owner, layout, tabs, tab, drafts) =
            shell.workspace.read_with(cx, |workspace, cx| {
                (
                    workspace.settings_shown(cx),
                    workspace.sheet_owner(),
                    workspace.layout().clone(),
                    workspace.tab_keys(),
                    workspace.shown_tab(),
                    workspace.drafts_shown(cx),
                )
            });
        Footprint {
            shown,
            owner,
            sheet,
            dialog,
            layout,
            tabs,
            tab,
            drafts,
            focus,
            requests: shell.requests(cx, |_| true).len(),
        }
    };
    let before = footprint(cx);
    assert_eq!(before.owner, Some(SheetOwnerKind::Settings));
    let actions: [(&str, Box<dyn Action>); 15] = [
        ("FocusRepositories", FocusRepositories.boxed_clone()),
        ("FocusNavigation", FocusNavigation.boxed_clone()),
        ("FocusContent", FocusContent.boxed_clone()),
        ("FocusOperations", FocusOperations.boxed_clone()),
        ("NextRegion", NextRegion.boxed_clone()),
        ("PreviousRegion", PreviousRegion.boxed_clone()),
        ("ToggleLeftDock", ToggleLeftDock.boxed_clone()),
        ("ToggleRightDock", ToggleRightDock.boxed_clone()),
        ("ToggleBottomDock", ToggleBottomDock.boxed_clone()),
        ("NewItem", NewItem.boxed_clone()),
        ("NextTab", NextTab.boxed_clone()),
        ("PreviousTab", PreviousTab.boxed_clone()),
        ("CloseTab", CloseTab.boxed_clone()),
        ("OpenDrafts", OpenDrafts.boxed_clone()),
        ("OpenSettings", OpenSettings.boxed_clone()),
    ];
    for (name, action) in actions {
        shell
            .visual(cx)
            .update(|window, cx| window.dispatch_action(action, cx));
        shell.run(cx);
        assert_eq!(footprint(cx), before, "{name} changed something");
    }
}

/// P8 (B2, R-12, R-40; Part G item 1): PageDown from a menu entry and End
/// from a page control move Settings' own offset, not the content column's
/// or the item's; Tab to a control below the fold reveals it in Settings.
#[gpui_kit::test]
fn settings_scrolls_on_its_own(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, long_and_other);
    shell.resize_keeping_content(1440., SHORT, cx);
    shell.set_activity_height(SHORT_ACTIVITY, cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-3", cx);
    shell.keys("pagedown", cx);
    let item = item_offset(&shell, cx);
    assert!(item < px(0.), "the item's region scrolled: {item:?}");
    let column = column_offset(&shell, cx);

    shell.keys("secondary-,", cx);
    assert_eq!(settings_offset(&shell, cx), px(0.));
    // PageDown, from the page's entry in the menu.
    shell.keys("pagedown", cx);
    let paged = settings_offset(&shell, cx);
    assert!(paged < px(0.), "PageDown scrolled Settings: {paged:?}");
    assert_eq!(column_offset(&shell, cx), column, "the column stays");
    assert_eq!(item_offset(&shell, cx), item, "the item stays");
    shell.keys("home", cx);
    assert_eq!(settings_offset(&shell, cx), px(0.), "Home, the top");
    // End, from a page control in view: the Add form's Browse.
    let host = host(&shell, cx);
    let browse = shell.visual(cx).update(|window, cx| {
        let page = host.update(cx, |host, cx| host.repositories(window, cx));
        page.read(cx).control_focus("add-browse")
    });
    shell.frame(cx, |window, cx| window.focus(&browse, cx));
    assert_eq!(settings_offset(&shell, cx), px(0.), "Browse is in view");
    shell.keys("end", cx);
    let bottom = settings_offset(&shell, cx);
    assert!(bottom < px(0.), "End scrolled Settings: {bottom:?}");
    assert_eq!(column_offset(&shell, cx), column);
    assert_eq!(item_offset(&shell, cx), item);
    shell.keys("home", cx);
    assert_eq!(settings_offset(&shell, cx), px(0.), "Home, the top");

    // Tab to a control below the fold: Settings brings it into view.
    tab_to_create(&shell, cx);
    let (viewport, submit) = shell.frame(cx, |window, _| {
        (
            window.find("settings-scroll").bounds(),
            window.find("create-submit").bounds(),
        )
    });
    assert!(
        submit.intersects(&viewport),
        "the focused control is in view: {submit:?} {viewport:?}"
    );
    assert!(settings_offset(&shell, cx) < px(0.));
    assert_eq!(column_offset(&shell, cx), column);
    assert_eq!(item_offset(&shell, cx), item);
}

/// P14 (C3, R-20, decision 5; A6 F11): with Activity hidden, Show in
/// Activity closes Settings, shows Activity and focuses its list, still so
/// after a draw.
#[gpui_kit::test]
async fn show_in_activity_closes_settings_and_focuses_activity(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.answer(
        0,
        Answer::Repository {
            recovery_required: true,
        },
        cx,
    );
    let shell = &rig.shell;
    shell.click(Control::Panel(PanelKind::Activity).id(), cx);
    assert!(
        !displayed(shell, PanelKind::Activity, cx),
        "Activity hidden"
    );
    rig.open(1, cx).await;
    let row = rig.row("a", cx);
    shell.click(("polling-pending", row), cx);
    closed(shell, "Show in Activity", cx);
    assert!(
        displayed(shell, PanelKind::Activity, cx),
        "Activity is shown"
    );
    let list = activity_list(shell, cx);
    assert!(holds_handle(shell, &list, cx), "F11: the Activity list");
    assert!(holds_handle(shell, &list, cx), "still so after a draw");
}

/// P15 (R-9, R-29): a route raised under an open dialog, and one raised
/// while quitting, open nothing.
#[gpui_kit::test]
async fn settings_does_not_open_behind_a_dialog_or_while_quitting(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
    });

    // Under a dialog: Share…'s route to Remotes.
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
    route(&shell, cx, |workspace, window, cx| {
        workspace.open_remotes(root.clone(), window, cx)
    });
    assert_eq!(shown(&shell, cx), Shown::Tabs, "under a dialog");
    assert_eq!(owner(&shell, cx), None);
    assert!(!has_sheet(&shell, cx));
    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);

    // While quitting: the footer's route to Checking for updates.
    let (held, id) = hold_a_job(&shell, &root, cx);
    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Finishing(id));
    route(&shell, cx, |workspace, window, cx| {
        workspace.open_settings(SettingsPage::Polling, window, cx)
    });
    assert_eq!(shown(&shell, cx), Shown::Tabs, "while quitting");
    assert_eq!(owner(&shell, cx), None);
    assert!(!has_sheet(&shell, cx));
    assert!(holds(&shell, "quit-now", cx), "Quit now keeps focus");

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
