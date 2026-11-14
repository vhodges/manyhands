//! What the popover's tests share: the popover's state as the Workspace
//! holds it, where focus is, and the footer item.

use gpui_kit::{FocusHandle, TestAppContext};

use crate::desktop::{
    model::activity::rows::{ActivityRow, RowKey},
    view::{footer::Control, harness::Shell},
};

/// The footer item's focus handle.
pub(in crate::desktop::view) fn item(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.footer_focus(Control::Attention)
    })
}

/// Whether the Workspace holds the popover open.
pub(in crate::desktop::view) fn is_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_open())
}

/// The keys of `rows`, in order.
pub(in crate::desktop::view) fn keys_of(rows: &[ActivityRow]) -> Vec<RowKey> {
    rows.iter().map(|row| row.row.key.clone()).collect()
}

/// The log's attention rows, newest first: what the footer counts and the
/// popover lists.
pub(in crate::desktop::view) fn attention_rows(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Vec<ActivityRow> {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .activity()
            .rows()
            .iter()
            .filter(|row| row.row.attention)
            .cloned()
            .collect()
    })
}

/// The rows the popover has applied, as drawn.
pub(in crate::desktop::view) fn applied(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Vec<ActivityRow> {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_rows())
}

/// The popover's highlighted row.
pub(in crate::desktop::view) fn cursor(shell: &Shell, cx: &mut TestAppContext) -> Option<RowKey> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_cursor())
}

/// Where focus is, as far as the popover is concerned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::desktop::view) enum Spot {
    /// The footer's Needs attention item.
    Item,
    /// The popover's list.
    List,
    /// The highlighted row's button.
    Button,
    /// "Show everything in Activity".
    Link,
    /// Anywhere else, or nowhere.
    Elsewhere,
}

/// Where focus is now, read in a frame.
pub(in crate::desktop::view) fn spot(shell: &Shell, cx: &mut TestAppContext) -> Spot {
    let [list, button, link] = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_handles());
    let item = item(shell, cx);
    shell.frame(cx, |window, _| {
        [
            (&item, Spot::Item),
            (&list, Spot::List),
            (&button, Spot::Button),
            (&link, Spot::Link),
        ]
        .into_iter()
        .find(|(handle, _)| handle.is_focused(window))
        .map_or(Spot::Elsewhere, |(_, spot)| spot)
    })
}

/// Focuses the footer item, as Tab would.
pub(in crate::desktop::view) fn focus_item(shell: &Shell, cx: &mut TestAppContext) {
    let item = item(shell, cx);
    shell.frame(cx, |window, cx| item.focus(window, cx));
    shell.run(cx);
}

/// The popover is drawn: its element is in the frame.
pub(in crate::desktop::view) fn drawn(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("needs-attention-popover", cx)
}

/// Opens the popover by a click on the footer item, and lets it draw.
pub(in crate::desktop::view) fn open_by_click(shell: &Shell, cx: &mut TestAppContext) {
    shell.click(Control::Attention.id(), cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
}

/// Lets the popover's rows follow the log and draws them.
pub(in crate::desktop::view) fn settle_popover(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}
