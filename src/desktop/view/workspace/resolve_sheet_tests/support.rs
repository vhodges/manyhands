//! What P6 and P7 share (Wave 04 Cycle 05b design B3, P6, P7): the canned
//! share conflict whose banner, Activity entry and popover row offer Review
//! changes; the reads of focus, each after two draws (design "Testing";
//! Cycle 04's A6 rule); and a table of rows that fails once, naming every
//! row that did not hold, so one run shows all of them.
//!
//! A conflict is canned (D5 decision 9), as `conflict/entry_tests.rs` cans
//! it: the share's answer is fabricated, and its read stands in place of the
//! library's (`answer_next_conflict_read`). Nothing is asserted of the
//! repository.

use std::path::{Path, PathBuf};

use gpui_kit::{
    App, Context, ElementId, Entity, FocusHandle, Focusable, TestAppContext, Window,
    component::{WindowExt, dialog::Dialog},
    test::TestWindowExt,
};
use manyhands::repository::{
    AuthoringKind, ConflictDto, ConflictPathDto, ConflictPathKind, ConflictPathStructure,
    ConflictReadEligibility, ConflictReadNextAction, ItemDtoKind, OperationId,
};

use super::super::super::{Region, Workspace, settings_sheet::SheetOwnerKind};
use crate::desktop::{
    model::{activity::rows::RowKey, navigation::OpenTarget},
    services::requests::{DesktopRequestId, Intent},
    test_support::fixtures::{Fixture, item_id, ticket_source},
    view::{
        conflict::{
            Opening, ResolveSheet, SheetSource,
            entry_tests::{SYNCHRONIZATION, fabricate, share_conflict},
            tests::{TICKET, canned, canned_path, opened, until_present},
        },
        harness::{Shell, key},
        lifecycle::Heading,
        settings::Shown,
    },
};

/// A second ticket of the repository, whose tab a landing can show in
/// place of the first (H6).
pub(super) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH1";
/// The banner's Review changes, its wrapper tracking the item's
/// `review_focus`: the origin of a sheet the banner opens.
pub(super) const BANNER: &str = "item-review-changes";

/// A window over repository `a`, its ticket `TICKET` shown in a tab and
/// saved once, so it has a work context of its own, and `OTHER` beside it;
/// a share of `TICKET` fabricated as stopped on a conflict of
/// `SYNCHRONIZATION`, so its banner, its Activity entry and its Needs
/// attention row offer Review changes.
pub(super) struct Rig {
    pub(super) shell: Shell,
    pub(super) root: PathBuf,
    pub(super) share: DesktopRequestId,
}

impl Rig {
    pub(super) fn new(cx: &mut TestAppContext) -> Self {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        fixture.commit_ticket(&root, TICKET, "Untitled", "");
        fixture.commit_ticket(&root, OTHER, "Other", "");
        fixture.refresh(&root);
        fixture.save_ticket(&root, TICKET, "Mine");
        fixture.refresh(&root);
        let shell = Shell::open(fixture, cx);
        open(&shell, &root, TICKET, cx);
        let share = fabricate(
            &shell,
            1,
            Intent::ItemSync {
                root: root.clone(),
                kind: AuthoringKind::Ticket,
                id: item_id(TICKET),
            },
            share_conflict(),
            cx,
        );
        Self { shell, root, share }
    }

    /// The share's Activity entry.
    pub(super) fn entry(&self) -> RowKey {
        RowKey::Request(self.share)
    }

    /// The banner's Review changes, clicked, with `read` standing in place
    /// of the library's: the sheet, once it has drawn `landed`. Focus is
    /// first put on the Repositories region, drawn and not the origin, so a
    /// close that only gives back what held focus as the sheet opened (the
    /// Kit's `close_sheet`) does not land on the origin.
    pub(super) async fn by_banner(
        &self,
        read: Option<ConflictDto>,
        landed: &'static str,
        cx: &mut TestAppContext,
    ) -> Entity<ResolveSheet> {
        until_present(&self.shell, BANNER, cx).await;
        if let Some(read) = read {
            self.shell.backend.answer_next_conflict_read(read);
        }
        let workspace = self.shell.workspace.clone();
        self.shell.visual(cx).update(|window, cx| {
            workspace.update(cx, |workspace, cx| {
                workspace.focus_region(Region::Repositories, window, cx)
            })
        });
        self.shell.run(cx);
        self.shell.click(BANNER, cx);
        until_present(&self.shell, landed, cx).await;
        opened(cx).expect("the sheet is open")
    }
}

/// Opens `id`'s tab in repository `root`.
pub(super) fn open(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

/// The share's operation.
pub(super) fn synchronization() -> OperationId {
    OperationId::parse(SYNCHRONIZATION).unwrap()
}

/// The share's conflict, live and to be resolved: `TICKET`'s title changed
/// on both sides, as `c4_conflict_read_canonical` reads it.
pub(super) fn canonical() -> ConflictDto {
    canned(synchronization(), vec![canned_path(TICKET, '1')])
}

/// The same, its title and body changed on both sides: a body part, with
/// its Result editor.
pub(super) fn with_body() -> ConflictDto {
    let mut path = canned_path(TICKET, '1');
    path.base = Some(ticket_source(TICKET, "Base", ""));
    path.local = Some(ticket_source(TICKET, "Local", ""));
    path.incoming = Some(ticket_source(TICKET, "Incoming", ""));
    canned(synchronization(), vec![path])
}

/// The same beside a binary file no item holds, so the whole set is
/// repaired outside, as `c4_conflict_read_noncanonical_or_structural` reads.
pub(super) fn external() -> ConflictDto {
    let binary = ConflictPathDto {
        display_path: "blob.bin".to_owned(),
        token: format!("conflict-path-v1:{}", "2".repeat(64)),
        kind: ConflictPathKind::Noncanonical,
        structure: ConflictPathStructure::Binary,
        content_available: false,
        base: None,
        local: None,
        incoming: None,
    };
    let mut read = canned(synchronization(), vec![canned_path(TICKET, '1'), binary]);
    read.next_action = ConflictReadNextAction::ExternalRepair;
    read.eligibility = Some(ConflictReadEligibility::ExternalResolutionRequired);
    read
}

/// A conflict whose commit is prepared, so Resume is offered, as
/// `c4_conflict_read_commit_prepared` reads.
pub(super) fn resumable() -> ConflictDto {
    let mut read = canonical();
    read.next_action = ConflictReadNextAction::Resume;
    read
}

/// What `show_resolve` opens on: the share's conflict of `item` in `root`,
/// from `source`, giving focus back to `origin`.
pub(super) fn opening(
    source: SheetSource,
    root: &Path,
    item: &str,
    origin: FocusHandle,
) -> Opening {
    Opening {
        source,
        root: root.to_owned(),
        operation_id: synchronization(),
        item: item_id(item),
        heading: Heading {
            kind: ItemDtoKind::Ticket,
            short: "ae-c10s3".to_owned(),
            title: Some("Mine".to_owned()),
            repository: "a".to_owned(),
            primary: Some("main".to_owned()),
            remote: Some(None),
        },
        folder: None,
        origin,
    }
}

/// Raises a way in as its trigger calls it, inside an update of the
/// workspace with its window, and gives what it gives.
pub(super) fn route<R>(
    shell: &Shell,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> R {
    let workspace = shell.workspace.clone();
    let given = shell
        .visual(cx)
        .update(|window, cx| workspace.update(cx, |workspace, cx| f(workspace, window, cx)));
    shell.run(cx);
    given
}

/// A dialog over whatever is open, as a prompt opens over it.
pub(super) fn open_probe(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog: Dialog, _, _| dialog.title("A probe"));
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

/// Resolve conflict is open: it owns the sheet, and the sheet is active.
pub(super) fn resolve_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx) == Some(SheetOwnerKind::Resolve) && has_sheet(shell, cx)
}

/// Settings is open: it owns the sheet, and the sheet is active.
pub(super) fn settings_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx) == Some(SheetOwnerKind::Settings) && has_sheet(shell, cx)
}

/// Nothing owns a sheet, none is active and no dialog is open.
pub(super) fn all_closed(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx).is_none() && !has_sheet(shell, cx) && !has_dialog(shell, cx)
}

/// Closes every dialog and sheet left open, so a row that failed does not
/// leave the next one under it. Not by Escape: focus a failed row left
/// behind the sheet would not reach it. The owner a closed sheet leaves is
/// cleared by `sync_sheet` as it draws.
pub(super) fn close_everything(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..4 {
        if has_dialog(shell, cx) {
            shell
                .visual(cx)
                .update(|window, cx| window.close_dialog(cx));
            shell.run(cx);
        } else if has_sheet(shell, cx) {
            shell.visual(cx).update(|window, cx| window.close_sheet(cx));
            shell.run(cx);
        }
    }
    settle(shell, cx);
}

/// Two draws, with what each starts run after it: a focus a row names is
/// read after them, so a `focus_lost` or a render-time rule that moved it
/// on the first has had its frame.
pub(super) fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

/// The innermost element that says it holds focus, after two draws. A
/// snapshot's `focused()` asserts on an element whose focus is not
/// observed, so its `Debug`, which says so instead, is read.
pub(super) fn focused_id(shell: &Shell, cx: &mut TestAppContext) -> Option<ElementId> {
    settle(shell, cx);
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| format!("{element:?}").contains("focused: Some(true)"))
            .max_by_key(|element| element.path().len())
            .and_then(|element| element.path().last().cloned())
    })
}

/// Whether focus is on the element `id` itself, not on something inside
/// it or around it, after two draws.
pub(super) fn on(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    focused_id(shell, cx) == Some(id.into())
}

/// Whether `handle` holds focus, after two draws.
pub(super) fn on_handle(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| handle.is_focused(window))
}

/// The sheet's own handle, which `conflict-sheet` tracks.
pub(super) fn own_focus(sheet: &Entity<ResolveSheet>, cx: &mut TestAppContext) -> FocusHandle {
    sheet.read_with(cx, |sheet, _| sheet.own_focus().clone())
}

/// Whether the sheet's own handle holds focus, after two draws.
pub(super) fn on_own(shell: &Shell, sheet: &Entity<ResolveSheet>, cx: &mut TestAppContext) -> bool {
    let handle = own_focus(sheet, cx);
    on_handle(shell, &handle, cx)
}

/// Whether the element `id`, or something in it, holds focus, after two
/// draws.
pub(super) fn within(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the Kit's close button, in the sheet's title row, holds focus.
pub(super) fn on_close_button(shell: &Shell, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| {
        window
            .within("sheet-content")
            .try_find("close")
            .and_then(|close| close.focused())
            == Some(true)
    })
}

/// Clicks the Kit's close button, in the sheet's title row.
pub(super) fn click_close_button(shell: &Shell, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| {
        window.within("sheet-content").click("close", cx)
    });
    shell.run(cx);
}

/// Puts focus on `handle`, as Tab or a click would.
pub(super) fn focus(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| window.focus(handle, cx));
    shell.run(cx);
}

/// The Result editor of the sheet's first body, focused, its caret at its
/// end. Gives the editor's own handle.
pub(super) fn into_the_editor(
    shell: &Shell,
    sheet: &Entity<ResolveSheet>,
    cx: &mut TestAppContext,
) -> FocusHandle {
    let editor = sheet
        .read_with(cx, |sheet, _| sheet.result_editor(0))
        .expect("the Result editor");
    let end = editor.read_with(cx, |editor, _| editor.text().len());
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    focus(shell, &handle, cx);
    shell.frame(cx, |_, _| ());
    handle
}

/// The region that holds focus, after two draws.
pub(super) fn region(shell: &Shell, cx: &mut TestAppContext) -> Option<Region> {
    settle(shell, cx);
    let workspace = shell.workspace.clone();
    shell.frame(cx, |window, cx| workspace.read(cx).held_region(window, cx))
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

/// The rows of one test, as they are read. A row whose read does not hold
/// is kept, with where focus was instead, and the table fails once at its
/// end, naming every such row: one run shows each row that is not yet met.
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
    /// `wanted`. A row that does not hold is kept with where focus is.
    pub(super) fn row(
        &mut self,
        shell: &Shell,
        row: &str,
        held: bool,
        wanted: &str,
        cx: &mut TestAppContext,
    ) {
        eprintln!("{}: {row}: {held}", self.name);
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
        eprintln!("{}: {row}: {held}", self.name);
        if held {
            self.held.push(row.to_owned());
        } else {
            self.failed.push(format!("{row}: {what} does not hold"));
        }
    }

    /// Fails, naming every row that did not hold, unless none did. Each row
    /// is printed as it is read, so a setup that panics shows how far the
    /// table got.
    pub(super) fn done(self) {
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

/// Where focus is, in words: the innermost element holding it, the region
/// that does, whether a sheet or a dialog is open, and the key contexts
/// down to it.
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
