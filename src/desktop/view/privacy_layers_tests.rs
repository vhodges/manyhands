//! What D5's and D6's privacy sweeps share about the layers of Wave 04 Cycle
//! 03b (Part G, Task 12, E-14): the Needs attention popover and a row's menu.
//!
//! An item's title, and only its title, may show in Activity's rows
//! (decisions 12 and 21), and so in the popover's rows, which are the same
//! rows, and in a row's menu, whose first item is the row's full message.
//! Nothing else of an item may: the body, a comment, a conflict side, a
//! repository's folder, a remote's path and user-info stay out of both.
//!
//! The popover has its own ids (`needs-attention-popover`, its rows and
//! buttons), so a sweep names it by them. Every Kit menu has the same id,
//! `"popup-menu"` (fact 33), so no id says whose menu is drawn: the row menu
//! is the drawn menu only while the Workspace says it is open
//! (`activity_menu_open()`, and one layer is open at a time, R-13), and
//! only then does the title allowance reach the items under that id. The
//! dock, repository and More menus are `"drawn"`, with no allowance.
//!
//! What no snapshot reads is read through the functions that build it: the
//! popover rows' tooltips and accessible descriptions (`tooltip`), its fixed
//! copy, and the row menu's items from the row the menu was built from.

use gpui_kit::{ElementId, TestAppContext};

use super::{
    activity::row::tooltip,
    footer::ATTENTION_TOOLTIP,
    harness::Shell,
    needs_attention::tests::support::{drawn, open_by_click, settle_popover},
};
use crate::desktop::model::activity::copy::{
    NEEDS_ATTENTION_SAFE, NEEDS_ATTENTION_TITLE, SHOW_EVERYTHING, message_of,
};

/// The Kit's id for every popup menu.
const POPUP_MENU: &str = "popup-menu";

/// The name of an element ID, its index aside.
fn name(id: &ElementId) -> Option<&str> {
    match id {
        ElementId::Name(name) | ElementId::NamedInteger(name, _) => Some(name),
        _ => None,
    }
}

/// Whether the element at `path` is drawn inside a menu that is the row's:
/// inside the Kit's `"popup-menu"` while `row_menu_open`. Any other menu
/// under that id (the dock's, the repository's, More's) is not, and the
/// title allowance does not reach it.
pub(in crate::desktop::view) fn in_row_menu(path: &[ElementId], row_menu_open: bool) -> bool {
    row_menu_open && path.iter().filter_map(name).any(|at| at == POPUP_MENU)
}

/// What the popover builds that no snapshot reads, as `(surface, text)`:
/// each applied row's tooltip lines and its accessible description (the
/// tooltip's lines joined), and the popover's fixed copy. Empty while the
/// popover is closed.
pub(in crate::desktop::view) fn popover_texts(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Vec<(&'static str, String)> {
    let rows = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_rows());
    if rows.is_empty() && !is_open(shell, cx) {
        return Vec::new();
    }
    let mut texts = Vec::new();
    for row in &rows {
        let lines = tooltip(row);
        texts.push(("popover description", lines.join(" ")));
        texts.extend(lines.into_iter().map(|line| ("popover tooltip", line)));
    }
    texts.extend(
        [
            NEEDS_ATTENTION_TITLE,
            NEEDS_ATTENTION_SAFE,
            SHOW_EVERYTHING,
            ATTENTION_TOOLTIP,
        ]
        .map(|copy| ("popover copy", copy.to_owned())),
    );
    texts
}

/// The open row menu's items, from the row it was built from: the full
/// message, the detail lines, then the offers' labels. The Kit reports no
/// disabled flag, so a disabled item is one of these too. Empty while no row
/// menu is open.
pub(in crate::desktop::view) fn row_menu_texts(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Vec<String> {
    shell.workspace.read_with(cx, |workspace, _| {
        let Some(row) = workspace
            .activity_menu_key()
            .and_then(|key| workspace.activity_pane().row(&key).cloned())
        else {
            return Vec::new();
        };
        std::iter::once(message_of(&row.row))
            .chain(row.row.detail.iter().cloned())
            .chain(row.row.menu.iter().map(|offer| offer.label.clone()))
            .collect()
    })
}

/// Whether the popover is open.
pub(in crate::desktop::view) fn is_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_open())
}

/// Opens the popover by a click on the footer item and lets its rows follow
/// the log and draw. It lists the rows that need attention.
pub(in crate::desktop::view) fn open_popover(shell: &Shell, cx: &mut TestAppContext) {
    open_by_click(shell, cx);
    settle_popover(shell, cx);
    assert!(
        drawn(shell, cx),
        "the popover is drawn: the sweep reads its elements"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A title is allowed only in the row's menu: the same menu id under
    /// the dock, repository or More menu is not, so a sentinel in one of
    /// those stays forbidden.
    #[test]
    fn the_allowance_reaches_only_the_row_menu() {
        let item = [
            ElementId::Name("workspace".into()),
            ElementId::Name(POPUP_MENU.into()),
            ElementId::Integer(0),
        ];
        assert!(in_row_menu(&item, true), "the row menu's item");
        assert!(!in_row_menu(&item, false), "any other menu's item");
        let elsewhere = [
            ElementId::Name("activity-row".into()),
            ElementId::Integer(0),
        ];
        assert!(!in_row_menu(&elsewhere, true), "not in a menu at all");
    }
}
