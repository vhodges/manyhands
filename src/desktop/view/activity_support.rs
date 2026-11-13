//! The test seam for Activity (Wave 04 Cycle 03b, Testing; R-21, R-45):
//! tests work by row key and offer label, not by element id.
//!
//! `act` chooses an offer through `Workspace::choose_activity`, the same
//! function a menu's choice calls, after a synchronous refresh of the log and
//! with the row's current revision. It dispatches no window action:
//! `Window::dispatch_action` runs deferred on the focused node, and
//! `FocusHandle::dispatch_action` does nothing for an undrawn handle. The
//! origin it passes is the list's handle, as the menu's and the button's
//! are (C3, decision 23).

use gpui_kit::{FocusHandle, TestAppContext, component::WindowExt};

use super::harness::Shell;
use crate::desktop::{
    keys::ActivityMenuChoice,
    model::{
        activity::rows::{ActivityAction, ActivityRow, Offer, RowKey},
        publication::SyncOutcome,
    },
    services::requests::{DesktopRequest, DesktopRequestId, RequestState},
};

/// The key of the highlighted row of the list, as drawn.
pub(crate) fn highlighted(shell: &Shell, cx: &mut TestAppContext) -> Option<RowKey> {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.activity_pane().highlighted_key().cloned()
    })
}

/// The list's focus handle.
pub(crate) fn list_focus(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.activity_pane().list_focus().clone()
    })
}

/// The highlighted row's button's focus handle.
pub(crate) fn button_focus(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.activity_pane().button_focus().clone()
    })
}

/// Whether the list holds focus, read in a frame.
pub(crate) fn list_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let list = list_focus(shell, cx);
    shell.frame(cx, |window, _| list.is_focused(window))
}

/// The rows the pane draws: what a person sees and presses.
pub(crate) fn drawn(shell: &Shell, cx: &mut TestAppContext) -> Vec<ActivityRow> {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_pane().rows().to_vec())
}

/// Where the row `key` is drawn, its element ids' index.
pub(crate) fn index_of(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) -> usize {
    drawn(shell, cx)
        .iter()
        .position(|row| &row.row.key == key)
        .unwrap_or_else(|| panic!("{key:?} is not drawn"))
}

/// Moves the highlight to `key` with the arrow keys, from wherever it is.
/// The list holds focus.
pub(crate) fn arrow_to(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) {
    let target = index_of(shell, key, cx);
    for _ in 0..drawn(shell, cx).len() {
        let at = highlighted(shell, cx).map(|current| index_of(shell, &current, cx));
        match at {
            Some(at) if at == target => return,
            Some(at) => shell.keys(if at < target { "down" } else { "up" }, cx),
            None => shell.keys("down", cx),
        }
    }
    assert_eq!(highlighted(shell, cx).as_ref(), Some(key));
}

/// Focuses Activity's list, as a person does (`secondary-4`), and moves to
/// the row `key` with the arrows, letting the pane draw it first.
pub(crate) fn to_row(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) {
    refresh(shell, cx);
    drawn(shell, cx);
    shell.keys("secondary-4", cx);
    arrow_to(shell, key, cx);
    assert!(list_focused(shell, cx), "the list holds focus on {key:?}");
}

/// Opens the menu of the row `key` by keyboard, as a person does: the list
/// focused on the row, `shift-f10`, then a click on its item `label`. For
/// what no row has as its button, such as Abandon… and Dismiss.
pub(crate) fn choose_in_menu(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) {
    to_row(shell, key, cx);
    shell.keys("shift-f10", cx);
    super::repositories::tests::support::choose(shell, label, cx);
}

/// The log's row `key`, as it stands.
pub(crate) fn row(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) -> Option<ActivityRow> {
    rows(shell, cx).into_iter().find(|row| &row.row.key == key)
}

/// The log's rows, newest first, as they stand.
pub(crate) fn rows(shell: &Shell, cx: &mut TestAppContext) -> Vec<ActivityRow> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity().rows().to_vec())
}

/// The offer of the row `key` labelled `label`, enabled or not.
pub(crate) fn offered(
    shell: &Shell,
    key: &RowKey,
    label: &str,
    cx: &mut TestAppContext,
) -> Option<Offer> {
    row(shell, key, cx)?
        .row
        .menu
        .into_iter()
        .find(|offer| offer.label == label)
}

/// Waits until the row `key` offers `label`, enabled.
pub(crate) async fn until_offered(
    shell: &Shell,
    key: &RowKey,
    label: &str,
    cx: &mut TestAppContext,
) {
    shell
        .until_drawn(cx, label, |shell, cx| {
            refresh(shell, cx);
            offered(shell, key, label, cx).is_some_and(|offer| offer.enabled)
        })
        .await;
}

/// The finished or running request `id`.
pub(crate) fn request(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> DesktopRequest {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).cloned())
        .unwrap_or_else(|| panic!("the model holds no request {id:?}"))
}

/// The `SyncOutcome` of the finished share `id`. It panics unless it has one.
pub(crate) fn share_outcome(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> SyncOutcome {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.share_result(id).cloned())
        .unwrap_or_else(|| panic!("{id:?} is no finished share"))
        .outcome
}

/// The label of the offer on the row `key` that runs `action`, as the log
/// stands after a refresh. It panics unless the row offers it.
pub(crate) fn label_of(
    shell: &Shell,
    key: &RowKey,
    action: &ActivityAction,
    cx: &mut TestAppContext,
) -> String {
    refresh(shell, cx);
    row(shell, key, cx)
        .unwrap_or_else(|| panic!("no row {key:?}"))
        .row
        .menu
        .into_iter()
        .find(|offer| &offer.action == action)
        .unwrap_or_else(|| panic!("{key:?} does not offer {action:?}"))
        .label
}

/// The key of the pending row of repository `root` whose command is `action`.
/// It panics unless exactly one is listed.
pub(crate) fn pending_key(
    shell: &Shell,
    root: &str,
    action: &str,
    cx: &mut TestAppContext,
) -> RowKey {
    let mut found: Vec<RowKey> = shell.operations(cx).read_with(cx, |operations, _| {
        let mut found = Vec::new();
        for (listed, state) in operations.pending_shown() {
            let Some(list) = state.rows().filter(|_| listed == root) else {
                continue;
            };
            for (at, operation) in list.items.iter().enumerate() {
                if operation.action.as_str() == action {
                    found.push(RowKey::Pending {
                        root: listed.clone(),
                        operation: operation
                            .operation_id
                            .clone()
                            .unwrap_or_else(|| format!("#{at}")),
                    });
                }
            }
        }
        found
    });
    assert_eq!(found.len(), 1, "one pending {action} row in {root}");
    found.remove(0)
}

/// Waits until the pending list of repository `root` holds an operation whose
/// command is `action`, and returns its row's key.
pub(crate) async fn until_pending(
    shell: &Shell,
    root: &str,
    action: &str,
    cx: &mut TestAppContext,
) -> RowKey {
    shell
        .until_drawn(cx, "the pending row", |shell, cx| {
            shell.operations(cx).read_with(cx, |operations, _| {
                operations
                    .pending_shown()
                    .into_iter()
                    .filter(|(listed, _)| listed == root)
                    .filter_map(|(_, state)| state.rows().map(|list| list.items.clone()))
                    .flatten()
                    .any(|operation| operation.action.as_str() == action)
            })
        })
        .await;
    pending_key(shell, root, action, cx)
}

/// Chooses the offer of the row `key` labelled `label`.
///
/// It cannot pass vacuously: the log is refreshed first, and it panics
/// unless exactly one enabled offer in the row's menu has that label. The
/// menu holds every offer, the button's included, so the button is not
/// counted twice.
pub(crate) fn act(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) {
    press(shell, key, label, true, cx);
}

/// `act` without the run that follows the choice, for a test that asserts
/// what has not happened yet, inside the choice's own handler.
pub(crate) fn act_unsettled(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) {
    press(shell, key, label, false, cx);
}

fn press(shell: &Shell, key: &RowKey, label: &str, settle: bool, cx: &mut TestAppContext) {
    refresh(shell, cx);
    let row = row(shell, key, cx).unwrap_or_else(|| panic!("no row {key:?} to choose {label} on"));
    let enabled: Vec<&Offer> = row
        .row
        .menu
        .iter()
        .filter(|offer| offer.enabled && offer.label == label)
        .collect();
    assert_eq!(
        enabled.len(),
        1,
        "exactly one enabled offer {label:?} on {key:?}: {:?}",
        row.row.menu
    );
    let choice = ActivityMenuChoice {
        key: key.clone(),
        action: enabled[0].action.clone(),
        shown: label.to_owned(),
        revision: row.revision,
    };
    choose(shell, choice, settle, cx);
}

/// Asserts the row `key` has no enabled offer labelled `label`, and that
/// choosing a disabled one, if the row has it, runs nothing (T12).
pub(crate) fn act_refused(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) {
    refresh(shell, cx);
    let row = row(shell, key, cx);
    let offers: Vec<Offer> = row
        .iter()
        .flat_map(|row| row.row.menu.iter())
        .filter(|offer| offer.label == label)
        .cloned()
        .collect();
    assert!(
        offers.iter().all(|offer| !offer.enabled),
        "{label:?} is offered on {key:?}"
    );
    let before = footprint(shell, cx);
    for offer in offers {
        let choice = ActivityMenuChoice {
            key: key.clone(),
            action: offer.action,
            shown: offer.label,
            revision: row.as_ref().expect("the row offers it").revision,
        };
        choose(shell, choice, true, cx);
    }
    assert_eq!(
        footprint(shell, cx),
        before,
        "choosing {label:?} on {key:?} ran something"
    );
}

/// The log, derived now: the refresh a deferred one would have made.
pub(crate) fn refresh(shell: &Shell, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.refresh_activity_now(cx));
}

/// Calls `choose_activity` with `choice`, inside `workspace.update`, with the
/// list's handle as the origin, as a menu's choice and a button give it
/// (C3).
fn choose(shell: &Shell, choice: ActivityMenuChoice, settle: bool, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            let origin = workspace.activity_pane().list_focus().clone();
            workspace.choose_activity(&choice, origin, window, cx)
        })
    });
    if settle {
        shell.run(cx);
    }
}

/// What a choice that runs something would change: the requests and their
/// states, a dialog, the page the content shows and the tabs.
fn footprint(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let mut seen: Vec<String> = shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .map(|request| {
                format!(
                    "{:?} attempt {} {}",
                    request.id,
                    request.attempt,
                    state_word(&request.state)
                )
            })
            .collect()
    });
    let dialog = shell
        .visual(cx)
        .update(|window, cx| window.has_active_dialog(cx));
    let (shown, tabs, drafts) = shell.workspace.read_with(cx, |workspace, cx| {
        (
            format!("{:?}", workspace.settings_shown(cx)),
            workspace.tab_keys().len(),
            workspace.drafts_shown(cx),
        )
    });
    seen.push(format!(
        "dialog {dialog}, {shown}, {tabs} tabs, drafts {drafts}"
    ));
    seen
}

/// A request's state in a word, with no text of the request in it.
fn state_word(state: &RequestState) -> String {
    match state {
        RequestState::Queued => "queued".to_owned(),
        RequestState::Running { stage, accepted } => format!("running {stage:?} {accepted}"),
        RequestState::Finished(outcome, code, ..) => {
            format!("finished {outcome:?} {}", code.as_str())
        }
        RequestState::Removed => "removed".to_owned(),
        RequestState::Dismissed => "dismissed".to_owned(),
        RequestState::NotSent(why) => format!("not sent {why:?}"),
    }
}
