//! What the Sharing & keys tests share (Wave 04 Cycle 04, C2): the
//! two-draw settle a focus design A6 names is read after (Ruling 2), a
//! table of rows that fails once, naming every row that did not hold, and
//! a press that a row finds missing says so.

use gpui_kit::{ElementId, FocusHandle, TestAppContext, test::TestWindowExt};

use crate::desktop::view::harness::Shell;

/// Two draws, with what each starts run after it: a focus read after them
/// is one design A6 counts.
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

/// Where focus is, in words: the innermost observed element that holds
/// it, or that nothing does.
pub(super) fn where_focus(shell: &Shell, cx: &mut TestAppContext) -> String {
    shell.frame(cx, |window, cx| {
        if window.focused(cx).is_none() {
            return "nothing holds focus".to_owned();
        }
        // A snapshot's `focused()` asserts on an element whose focus is not
        // observed, so its `Debug`, which says so instead, is read.
        let held = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| format!("{element:?}").contains("focused: Some(true)"))
            .max_by_key(|element| element.path().len())
            .and_then(|element| element.path().last().cloned());
        format!("focus is in {held:?}")
    })
}

/// The rows of a table test, as they are read: a row that does not hold is
/// kept with what was found, and the table fails once at its end, naming
/// every such row. No row's text holds a typed secret.
pub(super) struct Rows {
    name: &'static str,
    held: usize,
    failed: Vec<String>,
}

impl Rows {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            held: 0,
            failed: Vec::new(),
        }
    }

    /// The row `row`: `held` says whether it holds; `what` is what was
    /// found when it does not.
    pub(super) fn fact(&mut self, row: &str, held: bool, what: impl std::fmt::Display) {
        if held {
            self.held += 1;
        } else {
            self.failed.push(format!("{row}: {what}"));
        }
    }

    /// Fails, naming every row that did not hold, unless none did.
    pub(super) fn done(self) {
        assert!(
            self.failed.is_empty(),
            "{}: {} of {} rows did not hold:\n  {}",
            self.name,
            self.failed.len(),
            self.failed.len() + self.held,
            self.failed.join("\n  ")
        );
    }
}

/// Clicks the control `id`, inside the sheet, when it is drawn. A row that
/// finds it missing says so, and gets false.
pub(super) fn press(
    shell: &Shell,
    rows: &mut Rows,
    row: &str,
    id: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    if !shell.present(id.clone(), cx) {
        rows.fact(row, false, format!("{id:?} is not drawn"));
        return false;
    }
    shell.click(id, cx);
    true
}

/// Presses Tab until the control `id` holds focus, then presses Enter, as
/// a keyboard does. A row whose Tab never reaches it says so, and gets
/// false.
pub(super) fn tab_and_enter(
    shell: &Shell,
    rows: &mut Rows,
    row: &str,
    id: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    if !shell.tab_reaches(id.clone(), TABS, cx) {
        rows.fact(row, false, format!("Tab never reached {id:?}"));
        return false;
    }
    shell.press("enter", cx);
    true
}

/// Tab presses enough to go round the whole sheet.
const TABS: usize = 60;
