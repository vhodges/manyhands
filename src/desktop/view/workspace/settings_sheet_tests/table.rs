//! A table of design A6's rows, for the tests that write them first (Task
//! 2's design checkpoint). Every row's focus is read after two draws
//! (Ruling 2), by the reads of `support`. A row whose read does not hold is
//! kept, with where focus was instead, and the table fails once at its end,
//! naming every such row: one run shows each row that is not yet met. A
//! setup step that does not hold still panics where it is: that row is not
//! expressed yet.

use gpui_kit::{TestAppContext, component::WindowExt};

use crate::desktop::view::harness::Shell;

/// The rows of one test, as they are read.
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

/// Where focus is, in words, read in a frame: whether anything holds it,
/// the region that does, and the key contexts down to it.
pub(super) fn where_focus(shell: &Shell, cx: &mut TestAppContext) -> String {
    let workspace = shell.workspace.clone();
    shell.frame(cx, |window, cx| {
        let region = workspace.read(cx).held_region(window, cx);
        let contexts: Vec<String> = window
            .context_stack()
            .iter()
            .map(|context| format!("{context:?}"))
            .collect();
        let sheet = window.has_active_sheet(cx);
        let dialog = window.has_active_dialog(cx);
        let nothing = if window.focused(cx).is_none() {
            "nothing focused, "
        } else {
            ""
        };
        // The innermost observed element that says it holds focus. A
        // snapshot's `focused()` asserts on an element whose focus is not
        // observed, so its `Debug`, which says so instead, is read.
        let held = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| format!("{element:?}").contains("focused: Some(true)"))
            .max_by_key(|element| element.path().len())
            .and_then(|element| element.path().last().cloned());
        format!(
            "{nothing}held by {held:?}, region {region:?}, sheet {sheet}, dialog {dialog}, \
             contexts [{}]",
            contexts.join(" > ")
        )
    })
}
