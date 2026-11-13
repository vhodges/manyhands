//! The two-repository window the shell's tests share, and its readings of
//! the Repositories panel: rows, marks, states, tooltips and the menu by
//! keyboard; and the elements of the repository details block Part F took
//! out of the item area.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::{
    Entity, FocusHandle, TestAppContext, VisualTestContext, Window, WindowHandle, component::Root,
    test::TestWindowExt,
};

use super::super::Workspace;
use crate::desktop::{
    keys,
    model::{navigation::Resolution, repository_state::NEEDS_FIXING_SENTENCE},
    open_test_window_with,
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
    view::repositories::{
        rows,
        tests::support::{highlight, menu_labels, press_and_hold_in},
    },
};

/// A window over a fixture with two repositories, `a` and `b`, whose
/// startup reads have run.
pub(super) struct Shell {
    pub(super) window: WindowHandle<Root>,
    pub(super) workspace: Entity<Workspace>,
    pub(super) backend: Arc<DelayingBackend>,
    pub(super) a: PathBuf,
    pub(super) b: PathBuf,
    pub(super) fixture: Fixture,
}

impl Shell {
    pub(super) fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        cx.update(keys::bind);
        let fixture = Fixture::new();
        let a = fixture.create_repository("a");
        let b = fixture.create_repository("b");
        let (window, backend) = cx.update(|cx| open_test_window_with(&fixture, cx));
        let workspace = window
            .update(cx, |root, _, _| {
                root.view().clone().downcast::<Workspace>().unwrap()
            })
            .unwrap();
        let shell = Self {
            window,
            workspace,
            backend,
            a,
            b,
            fixture,
        };
        shell.visual(cx).run_until_parked();
        shell
    }

    pub(super) fn visual(&self, cx: &TestAppContext) -> VisualTestContext {
        VisualTestContext::from_window(self.window.into(), cx)
    }

    /// Draws a frame and runs `f` over the window.
    pub(super) fn frame<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Window, &mut gpui_kit::App) -> R,
    ) -> R {
        let mut visual = self.visual(cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            f(window, cx)
        })
    }

    /// Clicks the element `id` and lets the reads it starts finish.
    pub(super) fn click(&self, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.frame(cx, |window, cx| window.click(id, cx));
        self.visual(cx).run_until_parked();
    }

    pub(super) fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        let mut visual = self.visual(cx);
        visual.simulate_keystrokes(keystrokes);
        visual.run_until_parked();
    }

    pub(super) fn resolution(&self, root: &Path, cx: &mut TestAppContext) -> Resolution {
        let key = key(root);
        self.workspace.read_with(cx, |workspace, _| {
            workspace
                .navigation()
                .entry(&key)
                .expect("a listed repository")
                .resolution
                .clone()
        })
    }

    pub(super) fn selected(&self, cx: &mut TestAppContext) -> Option<String> {
        self.workspace.read_with(cx, |workspace, _| {
            workspace.navigation().selected().cloned()
        })
    }

    pub(super) fn label(
        &self,
        id: impl Into<gpui_kit::ElementId>,
        cx: &mut TestAppContext,
    ) -> Option<String> {
        let id = id.into();
        self.frame(cx, |window, _| {
            window
                .try_find(id)
                .and_then(|element| element.label().map(str::to_owned))
        })
    }

    pub(super) fn present(
        &self,
        id: impl Into<gpui_kit::ElementId>,
        cx: &mut TestAppContext,
    ) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| window.try_find(id).is_some())
    }
}

pub(super) fn key(root: &Path) -> String {
    root.to_str().unwrap().to_owned()
}

/// Row `index` of the repository list, in the read's order.
pub(super) fn row(index: usize) -> (&'static str, usize) {
    ("repo-row", index)
}

/// The action of the repository list's alert row: Retry.
pub(super) fn alert_action() -> gpui_kit::ElementId {
    (gpui_kit::ElementId::from("repositories-alert"), "action").into()
}

/// The chosen repository's mark on row `index`.
pub(super) fn mark(index: usize) -> (&'static str, usize) {
    ("repo-selected", index)
}

/// What row `index` says at its right, if anything.
pub(super) fn state(index: usize) -> (&'static str, usize) {
    ("repo-state", index)
}

/// Where the row of `root` is drawn, the kept row last.
pub(super) fn row_of(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> usize {
    shell.frame(cx, |_, _| ());
    shell.workspace.read_with(cx, |workspace, cx| {
        rows(workspace, cx)
            .iter()
            .position(|row| row.key == key(root))
            .expect("a row of the repository")
    })
}

/// `repository_tooltip`'s lines for the row of `root`.
pub(super) fn tooltip(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Vec<String> {
    shell.workspace.read_with(cx, |workspace, cx| {
        rows(workspace, cx)
            .into_iter()
            .find(|row| row.key == key(root))
            .expect("a row of the repository")
            .tooltip()
            .to_vec()
    })
}

/// Whether a line of `root`'s tooltip says its configuration needs fixing.
pub(super) fn needs_fixing(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> bool {
    tooltip(shell, root, cx)
        .iter()
        .any(|line| line.starts_with(NEEDS_FIXING_SENTENCE))
}

/// The Repositories list's focus handle: where its menu is opened from.
fn list_focus(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.repositories_list_focus(cx))
}

/// The menu of the row the list's cursor is on, by keyboard: the list,
/// then `shift-f10`. Its items' labels.
pub(super) fn open_menu(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.keys("secondary-1", cx);
    shell.keys("shift-f10", cx);
    shell.frame(cx, |window, _| menu_labels(window))
}

/// Chooses the open menu's item `label` by the arrows and Enter, and says
/// whether the list held focus as Enter chose it.
pub(super) fn choose_by_keys(shell: &Shell, label: &str, cx: &mut TestAppContext) -> bool {
    let list = list_focus(shell, cx);
    let mut visual = shell.visual(cx);
    highlight(&mut visual, label);
    press_and_hold_in(&mut visual, "enter", &list)
}

/// Every element the repository details block drew, in the item area or
/// at the end of an item tab, before Part F took it out.
fn details_block() -> Vec<gpui_kit::ElementId> {
    let mut ids: Vec<gpui_kit::ElementId> = [
        "selection-name",
        "selection-resolving",
        "selection-unavailable-state",
        "selection-unavailable",
        "selection-busy",
        "selection-failed",
        "selection-retry",
        "selection-remove",
        "selection-configuration",
        "selection-index",
        "selection-share-main",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    ids.push(("selection-problem", 0usize).into());
    ids
}

/// None of the details block's elements is drawn anywhere in the window.
pub(super) fn no_details(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        details_block()
            .into_iter()
            .all(|id| window.try_find(id).is_none())
    })
}
