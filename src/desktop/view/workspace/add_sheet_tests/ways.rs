//! P4, `each_way_in_opens_the_add_sheet` (design B2; G1): AW-1 to AW-5,
//! each through its own trigger, open the Add sheet in the existing-folder
//! mode (its segment chosen, B5) with focus on `add-path` after two draws.
//! The sheet is left in the new mode before the next open, and before an
//! `Inspect` route in place, so the mode is shown again by each. AW-5 is raised three
//! ways through Cycle 03b's `choose_activity`: an Activity row's button,
//! its row menu and a popover row. `Inspect` routes fill and check the
//! folder, also when the sheet is already open (B2 step 5: the origin
//! kept), and leave Settings' chosen repository alone (E-17c).
//!
//! The lifecycle answer (AW-4) and the recovery rows (AW-5) are canned in
//! the request model, as `lifecycle::outcome_canned_tests` and Activity's
//! own tests can them; no worker runs them.

use std::path::{Path, PathBuf};

use gpui_kit::{ElementId, FocusHandle, TestAppContext};
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleBlocker, LifecycleMadeReadyState, LifecycleMutationDto,
        LifecycleMutationType, MutationDataDto,
    },
    results::{Effects, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use super::support::*;
use crate::desktop::{
    model::{
        activity::{copy::OPEN_LIFECYCLE, rows::RowKey},
        operations::recovery_label,
        stamp::SettingsPage,
    },
    services::requests::{DesktopRequestId, Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity::tests::cases::{answered, fabricate},
        activity_support::{act, button_focus, choose_in_menu, list_focus, refresh, to_row},
        footer::Control,
        harness::{Shell, key},
        workspace::add_sheet::AddRoute,
    },
};

/// The ticket the canned lifecycle answer closes.
pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KC0";
/// The canned lifecycle request's number among the fabricated ones.
const CANNED: u64 = 70;

/// The recovery `repo.inspect` of `root`.
fn inspect(root: &Path) -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RepoInspect,
        [("root", key(root).into())],
    )
}

/// The Activity row's words for Inspect repository.
pub(super) fn inspect_label() -> &'static str {
    recovery_label(RecoveryActionKind::RepoInspect)
}

/// A rebuild of `root` that ended blocked and offers `repo.inspect` (AW-5),
/// put in the request model: its row's button is Inspect repository, and
/// it needs attention. Gives its request.
pub(super) fn inspect_row(
    shell: &Shell,
    n: u64,
    root: &Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let envelope =
        answered("index rebuild", ResultCode::RecoveryRequired).with_recovery(vec![inspect(root)]);
    fabricate(
        shell,
        n,
        Intent::IndexRebuild {
            root: root.to_owned(),
        },
        envelope,
        cx,
    )
}

/// The footer's Needs attention item.
pub(super) fn attention_item(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.footer_focus(Control::Attention)
    })
}

/// Opens the popover by Enter on the footer item, and presses its
/// highlighted row, which must be `row`, with Enter.
pub(super) fn press_in_popover(shell: &Shell, row: &RowKey, cx: &mut TestAppContext) {
    refresh(shell, cx);
    settle(shell, cx);
    let item = attention_item(shell, cx);
    focus(shell, &item, cx);
    shell.press("enter", cx);
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    let (open, cursor) = shell.workspace.read_with(cx, |workspace, _| {
        (
            workspace.attention_popover_open(),
            workspace.attention_popover_cursor(),
        )
    });
    assert!(open, "Enter on the footer item opened the popover");
    assert_eq!(cursor.as_ref(), Some(row), "the row is highlighted");
    shell.press("enter", cx);
    shell.run(cx);
}

/// A ticket committed and saved once, so it has a working copy to close.
pub(super) fn saved_ticket(fixture: &Fixture, root: &Path) {
    fixture.commit_ticket(root, TICKET, "Untitled", "");
    fixture.refresh(root);
    fixture.save_ticket(root, TICKET, "Ways");
}

/// AW-4: a close of `TICKET` blocked short of main, with a dirty main and
/// `repo.inspect`, its lifecycle dialog opened from Activity's Open…, and
/// Open repository details pressed. Gives Activity's list, the dialog's
/// origin, which it focuses before it routes.
pub(super) async fn open_repository_details(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> FocusHandle {
    let data = LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind: ItemDtoKind::Ticket,
        item_id: TICKET.to_owned(),
        recording: None,
        checkpoint_oid: None,
        integration: None,
        integration_oid: None,
        publication: None,
        publication_oid: None,
        publication_pending: None,
        cleanup: None,
        cleanup_pending: None,
        blockers: vec![LifecycleBlocker::PrimaryDirty],
        made_ready: Some(Vec::new()),
        made_ready_state: Some(LifecycleMadeReadyState::NotComputed),
    };
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        "ticket close",
        Scope {
            repository: Some(key(root)),
            item_id: Some(TICKET.to_owned()),
            ..Scope::default()
        },
        false,
        ResultCode::RecoveryRequired,
        Effects::not_requested(),
        false,
    );
    envelope.outcome = Outcome::Blocked;
    envelope.recovery = vec![inspect(root)];
    envelope.data = Some(MutationDataDto::Lifecycle(Box::new(data)));
    let intent = Intent::TicketClose {
        root: root.to_owned(),
        id: item_id(TICKET),
    };
    let id = fabricate(shell, CANNED, intent, envelope, cx);
    act(shell, &RowKey::Request(id), OPEN_LIFECYCLE, cx);
    shell
        .until_drawn(cx, "Open repository details", |shell, cx| {
            shell.present("lifecycle-inspect", cx)
        })
        .await;
    let origin = list_focus(shell, cx);
    shell.click("lifecycle-inspect", cx);
    origin
}

/// The empty Repositories panel's "Add or create a repository…" (AW-1).
pub(super) fn empty_action() -> ElementId {
    (ElementId::from("repositories-empty"), "action").into()
}

/// G1, its existing-folder mode included, and, for an `Inspect` route of
/// `root`, its folder filled and its check started. False when the sheet
/// did not open.
fn lands(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    inspected: Option<&Path>,
    cx: &mut TestAppContext,
) -> bool {
    if !opened(table, shell, row, cx) {
        return false;
    }
    table.fact(
        &format!("{row} (the existing-folder mode)"),
        shell.selected("add-mode-existing", cx) && !shell.selected("add-mode-new", cx),
        "the existing-folder mode shown",
    );
    if let Some(root) = inspected {
        let text = key(root);
        let filled = value(shell, "add-path", cx);
        table.fact(
            &format!("{row} (its folder)"),
            filled.as_deref() == Some(text.as_str()),
            &format!("add-path {text} (it is {filled:?})"),
        );
        let checking = of_sheet(shell, cx, |sheet| sheet.inspected().map(str::to_owned)).flatten();
        table.fact(
            &format!("{row} (its check)"),
            checking.as_deref() == Some(text.as_str()),
            &format!("a check of {text} started (it is {checking:?})"),
        );
    }
    let held = holds(shell, "add-path", cx);
    table.row(shell, &format!("{row}: G1"), held, "add-path", cx);
    true
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    // AW-1: the empty panel's action.
    let shell = Shell::open(Fixture::new(), cx);
    let drawn = shell.present(empty_action(), cx);
    table.fact("AW-1 (its action drawn)", drawn, "the empty panel's action");
    if drawn {
        shell.click(empty_action(), cx);
        lands(
            table,
            &shell,
            "AW-1 the empty panel's Add or create a repository…",
            None,
            cx,
        );
    }

    // AW-2: the toolbar's `+`, by Enter.
    let (shell, a, _) = shell_with_two(cx);
    press_plus(&shell, cx);
    lands(table, &shell, "AW-2 the toolbar's +", None, cx);
    // Left in the new mode: the next open shows the existing folder again.
    leave_in_the_new_mode(table, &shell, "AW-2", "enter", cx);
    shell.keys("escape", cx);
    if settings_open(&shell, cx) {
        shell.keys("escape", cx);
    }

    // B2 step 5: an Inspect route while the sheet is open applies in place
    // and keeps the origin.
    let plus = press_plus(&shell, cx);
    leave_in_the_new_mode(table, &shell, "B2 step 5", "space", cx);
    route(&shell, cx, {
        let a = a.clone();
        move |workspace, window, cx| {
            workspace.show_add_repository(AddRoute::Inspect(a), None, window, cx)
        }
    });
    let row = "B2 step 5 Inspect(root) while the sheet is open";
    let applied = add_open(&shell, cx) && lands(table, &shell, row, Some(&a), cx);
    if !applied {
        table.fact(
            &format!("{row} (opened by + first)"),
            false,
            "the sheet open by +",
        );
    }
    shell.keys("escape", cx);
    if applied {
        let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
        table.row(&shell, "B2 step 5 the origin kept", held, "+", cx);
    } else if settings_open(&shell, cx) {
        shell.keys("escape", cx);
    }

    // AW-3: Settings › Repositories' Add or create a repository….
    shell.keys("secondary-,", cx);
    let drawn =
        settings_on(&shell, SettingsPage::Repositories, cx) && shell.present(ADD_BUTTON.0, cx);
    table.fact(
        "AW-3 (its button drawn)",
        drawn,
        "Settings' Add or create a repository…",
    );
    if drawn {
        shell.click(ADD_BUTTON.0, cx);
        lands(
            table,
            &shell,
            "AW-3 Settings' Add or create a repository…",
            None,
            cx,
        );
    }
    shell.keys("escape", cx);
    if settings_open(&shell, cx) {
        shell.keys("escape", cx);
    }

    // AW-4: the lifecycle dialog's Open repository details.
    let (shell, root) = one_repo(cx, saved_ticket);
    open_repository_details(&shell, &root, cx).await;
    lands(
        table,
        &shell,
        "AW-4 the lifecycle dialog's Open repository details",
        Some(&root),
        cx,
    );
    shell.keys("escape", cx);

    // AW-5, three ways.
    for way in ["its button", "its row menu", "a popover row"] {
        let (shell, root) = one_repo(cx, |_, _| ());
        let id = inspect_row(&shell, 1, &root, cx);
        let row = RowKey::Request(id);
        match way {
            "its button" => {
                to_row(&shell, &row, cx);
                let button = button_focus(&shell, cx);
                focus(&shell, &button, cx);
                shell.press("enter", cx);
            }
            "its row menu" => choose_in_menu(&shell, &row, inspect_label(), cx),
            _ => press_in_popover(&shell, &row, cx),
        }
        lands(
            table,
            &shell,
            &format!("AW-5 a recovery's Inspect repository, by {way}"),
            Some(&root),
            cx,
        );
        shell.keys("escape", cx);
    }

    leaves_settings_chosen(table, cx);
}

/// The new mode chosen by its segment, focused and pressed with `key`
/// (B5, R-10: each segment is a Tab stop that Enter or Space chooses).
fn leave_in_the_new_mode(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    key: &str,
    cx: &mut TestAppContext,
) {
    if let Some(segment) = of_sheet(shell, cx, |sheet| sheet.control_focus("add-mode-new")) {
        focus(shell, &segment, cx);
        shell.press(key, cx);
    }
    table.fact(
        &format!("{row} (left in the new mode by {key})"),
        shell.selected("add-mode-new", cx) && !shell.selected("add-mode-existing", cx),
        "the new mode chosen",
    );
}

/// Repositories `a` and `b`, `a` selected in the panel. Gives the window,
/// `a` and `b`.
fn shell_with_two(cx: &mut TestAppContext) -> (Shell, PathBuf, PathBuf) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.refresh(&a);
    fixture.refresh(&b);
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    (shell, a, b)
}

/// P4's last claim and E-17c (R-55's, moved here): Settings works on `a`;
/// the panel then selects `b`, and `Inspect(b)` opens the Add sheet. Settings'
/// choice stays `a`, at that open and at the next plain open of Settings.
fn leaves_settings_chosen(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, a, b) = shell_with_two(cx);
    shell.keys("secondary-,", cx);
    assert_eq!(chosen(&shell, cx), Some(a.clone()), "Settings works on a");
    shell.keys("escape", cx);
    shell.select(&b, cx);
    route(&shell, cx, {
        let b = b.clone();
        move |workspace, window, cx| {
            workspace.show_add_repository(AddRoute::Inspect(b), None, window, cx)
        }
    });
    let row = "E-17c Inspect(b), the panel now on b";
    // Settings' choice is read whether or not the sheet opened: the sheet
    // records the panel's selection as Settings' own opens do (Ruling 15).
    let open = opened(table, &shell, row, cx);
    let found = chosen(&shell, cx);
    table.fact(
        &format!("{row}: Settings' choice"),
        found == Some(a.clone()),
        &format!("Settings still on a (it is {found:?})"),
    );
    if open {
        shell.keys("escape", cx);
    }
    shell.click(Control::Settings.id(), cx);
    let found = chosen(&shell, cx);
    table.fact(
        "E-17c the next plain open of Settings after Inspect(b)",
        found == Some(a),
        &format!("Settings still on a (it is {found:?})"),
    );
    shell.keys("escape", cx);
}
