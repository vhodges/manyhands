//! What the Add repository sheet's window tests share (Wave 04 Cycle 05a
//! design B9): the fixtures, the reads of the workspace and the sheet, the
//! two-draw settle a focus B9 names is read after (Cycle 04's A6 rule), and
//! a table of rows that fails once, naming every row that did not hold.
//!
//! A row first checks that the Add sheet opened (`opened`); when it did
//! not, the row is recorded as failed and goes no further, so one run shows
//! every row that is not met.

use std::path::{Path, PathBuf};

use gpui_kit::{
    Context, ElementId, Entity, FocusHandle, Modifiers, TestAppContext, Window,
    component::{WindowExt, input::InputState},
    point, px,
    test::TestWindowExt,
};

use super::super::super::settings_sheet::SheetOwnerKind;
use crate::desktop::{
    model::{navigation::OpenTarget, stamp::SettingsPage},
    services::requests::{DesktopRequestId, Intent, Lane, RequestState},
    test_support::fixtures::{Fixture, identity, item_id},
    view::{
        add_repository::AddRepositorySheet,
        confirm::PREPARING,
        harness::{Shell, key},
        settings::Shown,
        workspace::{Region, ToolbarButton, Workspace},
    },
};
use manyhands::repository::ItemDtoKind;

pub(super) const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW0";
pub(super) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW1";
/// Add repository's and Create repository's own Kit buttons, in their
/// wrappers `add-submit` and `create-submit`: G13's "the button that opened
/// it", read by `holds_control` (B12; Cycle 04 Ruling 55).
pub(super) const ADD_SUBMIT: (&str, usize) = ("add-submit", 0);
pub(super) const CREATE_SUBMIT: (&str, usize) = ("create-submit", 0);
/// Settings' "Add or create a repository…" (B11), its own Kit button in
/// its wrapper `settings-add-repository`: where G4 lands.
pub(super) const ADD_BUTTON: (&str, usize) = ("settings-add-repository", 0);
/// The sheet's footer Cancel (B10).
pub(super) const CANCEL: &str = "add-cancel";

/// One repository, `a`, built by `build` and refreshed, in an open window.
pub(super) fn one_repo(
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// Repository `a`, with a plain repository `plain` Git has an identity
/// for, which the Add sheet can add. Gives the window, `a` and `plain`.
pub(super) fn plain_shell(cx: &mut TestAppContext) -> (Shell, PathBuf, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.refresh(&root);
    let plain = fixture.plain_repository("plain", Some(identity()));
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    (shell, root, plain)
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

pub(super) fn chosen(shell: &Shell, cx: &mut TestAppContext) -> Option<PathBuf> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_chosen(cx))
}

/// The Add sheet the workspace keeps, once made.
pub(super) fn sheet(shell: &Shell, cx: &mut TestAppContext) -> Option<Entity<AddRepositorySheet>> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.add_sheet())
}

/// Reads `read` of the Add sheet, once it is made.
pub(super) fn of_sheet<R>(
    shell: &Shell,
    cx: &mut TestAppContext,
    read: impl FnOnce(&AddRepositorySheet) -> R,
) -> Option<R> {
    sheet(shell, cx).map(|sheet| sheet.read_with(cx, |sheet, _| read(sheet)))
}

pub(super) fn input(
    shell: &Shell,
    id: &str,
    cx: &mut TestAppContext,
) -> Option<Entity<InputState>> {
    of_sheet(shell, cx, |sheet| sheet.input(id))
}

pub(super) fn value(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Option<String> {
    let input = input(shell, id, cx)?;
    Some(input.read_with(cx, |input, _| input.value().to_string()))
}

/// Sets the field `id` to `text`, as typing it would. False when the sheet
/// is not made.
pub(super) fn set(shell: &Shell, id: &str, text: &str, cx: &mut TestAppContext) -> bool {
    let Some(input) = input(shell, id, cx) else {
        return false;
    };
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
    });
    shell.run(cx);
    true
}

/// Focuses the field `id` of the sheet.
pub(super) fn focus_in(shell: &Shell, id: &str, cx: &mut TestAppContext) {
    if let Some(input) = input(shell, id, cx) {
        shell.visual(cx).update(|window, cx| {
            input.update(cx, |input, cx| input.focus(window, cx));
        });
        shell.run(cx);
    }
}

/// Puts focus on the control the sheet's wrapper `id` tracks.
pub(super) fn focus_control(shell: &Shell, id: &str, cx: &mut TestAppContext) {
    if let Some(handle) = of_sheet(shell, cx, |sheet| sheet.control_focus(id)) {
        focus(shell, &handle, cx);
    }
}

/// Puts focus on `handle`, as Tab or a click would.
pub(super) fn focus(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| window.focus(handle, cx));
    shell.run(cx);
}

/// Two draws, with what each starts run after it: a focus B9 names is read
/// after them, so a `focus_lost` or a render-time rule that moved it on the
/// first has had its frame.
pub(super) fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

/// Whether the element `id`, or something in it, holds focus, after two
/// draws.
pub(super) fn holds(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether `handle` holds focus, after two draws.
pub(super) fn holds_handle(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| handle.is_focused(window))
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

/// The region that holds focus, after two draws.
pub(super) fn region(shell: &Shell, cx: &mut TestAppContext) -> Option<Region> {
    settle(shell, cx);
    let workspace = shell.workspace.clone();
    shell.frame(cx, |window, cx| workspace.read(cx).held_region(window, cx))
}

/// The Add sheet is open: it owns the sheet, a sheet is active, the
/// workspace holds the sheet, and Settings is closed.
pub(super) fn add_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx) == Some(SheetOwnerKind::AddRepository)
        && has_sheet(shell, cx)
        && sheet(shell, cx).is_some()
        && shown(shell, cx) == Shown::Tabs
}

/// Row `row`'s first fact: the Add sheet opened. False, and recorded, when
/// it did not; the row goes no further.
pub(super) fn opened(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) -> bool {
    let open = add_open(shell, cx);
    let what = format!(
        "the Add sheet open (owner {:?}, sheet {}, made {}, Settings {:?})",
        owner(shell, cx),
        has_sheet(shell, cx),
        sheet(shell, cx).is_some(),
        shown(shell, cx)
    );
    table.fact(&format!("{row} (the Add sheet opened)"), open, &what);
    open
}

/// Nothing owns a sheet, none is active and Settings is closed.
pub(super) fn all_closed(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx).is_none() && !has_sheet(shell, cx) && shown(shell, cx) == Shown::Tabs
}

/// Settings is open: it owns the sheet, and the sheet is active.
pub(super) fn settings_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx) == Some(SheetOwnerKind::Settings) && has_sheet(shell, cx)
}

/// Settings is open on `page`.
pub(super) fn settings_on(shell: &Shell, page: SettingsPage, cx: &mut TestAppContext) -> bool {
    settings_open(shell, cx) && shown(shell, cx) == Shown::Settings(page)
}

/// The Repositories toolbar's `+` (`add-repository-button`).
pub(super) fn plus(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .toolbar_focus(ToolbarButton::AddRepository)
            .clone()
    })
}

/// AW-2: the toolbar's `+`, focused and pressed with Enter. Gives `+`.
pub(super) fn press_plus(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    let plus = plus(shell, cx);
    focus(shell, &plus, cx);
    shell.press("enter", cx);
    plus
}

/// How a row closes the open sheet (B3): the Kit's three, and its Cancel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Way {
    Escape,
    CloseButton,
    Scrim,
    Cancel,
}

pub(super) const KEPT: [Way; 4] = [Way::Escape, Way::CloseButton, Way::Scrim, Way::Cancel];

/// Closes the open sheet `way`. False when the way's control is not drawn.
pub(super) fn close(shell: &Shell, way: Way, cx: &mut TestAppContext) -> bool {
    match way {
        Way::Escape => shell.keys("escape", cx),
        Way::CloseButton => {
            let drawn = shell.frame(cx, |window, _| {
                window.within("sheet-content").try_find("close").is_some()
            });
            if !drawn {
                return false;
            }
            shell.frame(cx, |window, cx| {
                window.within("sheet-content").click("close", cx)
            });
            shell.run(cx);
        }
        Way::Scrim => {
            // Left of the 470 px sheet, on its overlay.
            let mut visual = shell.visual(cx);
            visual.simulate_click(point(px(120.), px(600.)), Modifiers::default());
            visual.run_until_parked();
        }
        Way::Cancel => {
            if !shell.present(CANCEL, cx) {
                return false;
            }
            shell.click(CANCEL, cx);
        }
    }
    true
}

/// Raises a way in as its trigger calls it, inside an update of the
/// workspace with its window.
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

/// The folder `folder` typed in Add's folder field and checked by Enter;
/// focus is then put on Add repository. False when the sheet is not made.
pub(super) fn checked_folder(shell: &Shell, folder: &Path, cx: &mut TestAppContext) -> bool {
    if !set(
        shell,
        "add-path",
        folder.to_str().expect("a UTF-8 path"),
        cx,
    ) {
        return false;
    }
    focus_in(shell, "add-path", cx);
    shell.keys("enter", cx);
    // A check answered busy waits out its retry on the test clock (Cycle
    // 04 Ruling 52): `keys_landed` advances it.
    shell.keys_landed(cx);
    focus_control(shell, "add-submit", cx);
    true
}

/// Add repository pressed on a checked `folder`: its preview landed.
pub(super) fn preview(shell: &Shell, folder: &Path, cx: &mut TestAppContext) -> bool {
    if !checked_folder(shell, folder, cx) || !shell.present("add-submit", cx) {
        return false;
    }
    shell.click("add-submit", cx);
    preview_landed(shell, cx)
}

/// Whether the confirmation shows its landed preview. It draws Confirm,
/// disabled, while it prepares; a prepare answered busy retries on the
/// test clock (Cycle 04 Ruling 52), which `keys_landed` advances.
pub(super) fn preview_landed(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let state = |cx: &mut TestAppContext| shell.label("confirm-state", cx);
    if state(cx).as_deref() == Some(PREPARING) {
        shell.keys_landed(cx);
    }
    shell.present("confirm-ok", cx) && state(cx).as_deref() != Some(PREPARING)
}

/// The add requests of this session, newest first.
pub(super) fn enables(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoEnable { .. })
    })
}

/// Waits until the background refresh the worker submits after an add of
/// `root` has ended: the worker must not outlive the test.
pub(super) async fn until_onboarded(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let wanted = root.to_owned();
    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                request.lane == Lane::Background
                    && matches!(&request.intent, Intent::IndexRefresh { root } if *root == wanted)
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
}

/// Confirm on the open preview, then the add until it has ended. Gives the
/// request the press sent, if it sent one: never an earlier add's.
pub(super) async fn confirm_and_end(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Option<DesktopRequestId> {
    let before = enables(shell, cx);
    shell.click("confirm-ok", cx);
    let after = enables(shell, cx);
    let id = after.into_iter().find(|id| !before.contains(id))?;
    shell.until_ended(id, cx).await;
    Some(id)
}

/// The rows of one test, as they are read (Cycle 04's `table`). A row
/// whose read does not hold is kept, with where focus was instead, and the
/// table fails once at its end, naming every such row.
pub(super) struct Table {
    name: &'static str,
    held: Vec<String>,
    failed: Vec<String>,
}

impl Table {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            held: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// The row `row`, read after its draws: `held` says whether focus is on
    /// `wanted`.
    pub(super) fn row(
        &mut self,
        shell: &Shell,
        row: &str,
        held: bool,
        wanted: &str,
        cx: &mut TestAppContext,
    ) {
        if held {
            self.held.push(row.to_owned());
        } else {
            let at = where_focus(shell, cx);
            self.failed
                .push(format!("{row}: wanted {wanted}; focus is {at}"));
        }
    }

    /// A row that is not a focus read: `held` says whether `what` is so.
    pub(super) fn fact(&mut self, row: &str, held: bool, what: &str) {
        if held {
            self.held.push(row.to_owned());
        } else {
            self.failed.push(format!("{row}: {what} does not hold"));
        }
    }

    /// Fails, naming every row that did not hold, unless none did.
    pub(super) fn done(self) {
        eprintln!("{}: rows that held: {:?}", self.name, self.held);
        assert!(
            self.failed.is_empty(),
            "{}: {} of {} rows did not hold:\n  {}",
            self.name,
            self.failed.len(),
            self.failed.len() + self.held.len(),
            self.failed.join("\n  ")
        );
    }
}

/// Where focus is, in words: the region that holds it, the key contexts
/// down to it, and whether a sheet or a dialog is open.
pub(super) fn where_focus(shell: &Shell, cx: &mut TestAppContext) -> String {
    let workspace = shell.workspace.clone();
    shell.frame(cx, |window, cx| {
        let region = workspace.read(cx).held_region(window, cx);
        let contexts: Vec<String> = window
            .context_stack()
            .iter()
            .map(|context| format!("{context:?}"))
            .collect();
        let held = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| format!("{element:?}").contains("focused: Some(true)"))
            .max_by_key(|element| element.path().len())
            .and_then(|element| element.path().last().cloned());
        format!(
            "{}held by {held:?}, region {region:?}, sheet {}, dialog {}, contexts [{}]",
            if window.focused(cx).is_none() {
                "nothing focused, "
            } else {
                ""
            },
            window.has_active_sheet(cx),
            window.has_active_dialog(cx),
            contexts.join(" > ")
        )
    })
}
