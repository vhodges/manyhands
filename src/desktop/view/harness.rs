//! A main window over a fixture, for the views' tests: frames, clicks,
//! keys and the labels a test reads.

use std::{path::Path, sync::Arc};

use gpui_kit::{
    App, ElementId, Entity, TestAppContext, VisualTestContext, Window, WindowHandle,
    component::Root, test::TestWindowExt,
};

use super::workspace::Workspace;
use crate::desktop::{
    keys, open_test_window_with,
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

pub(crate) struct Shell {
    pub window: WindowHandle<Root>,
    pub workspace: Entity<Workspace>,
    pub backend: Arc<DelayingBackend>,
    pub fixture: Fixture,
}

impl Shell {
    /// Opens the main window over `fixture`, whose repositories are
    /// already made, and lets its startup reads finish.
    pub(crate) fn open(fixture: Fixture, cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        cx.update(keys::bind);
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
            fixture,
        };
        shell.run(cx);
        shell
    }

    pub(crate) fn visual(&self, cx: &TestAppContext) -> VisualTestContext {
        VisualTestContext::from_window(self.window.into(), cx)
    }

    /// Lets every started read finish.
    pub(crate) fn run(&self, cx: &TestAppContext) {
        self.visual(cx).run_until_parked();
    }

    /// Draws a frame and runs `f` over the window.
    pub(crate) fn frame<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Window, &mut App) -> R,
    ) -> R {
        let mut visual = self.visual(cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            f(window, cx)
        })
    }

    /// Clicks the element `id` and lets the reads it starts finish.
    pub(crate) fn click(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.frame(cx, |window, cx| window.click(id, cx));
        self.run(cx);
    }

    pub(crate) fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        let mut visual = self.visual(cx);
        visual.simulate_keystrokes(keystrokes);
        visual.run_until_parked();
    }

    pub(crate) fn label(
        &self,
        id: impl Into<ElementId>,
        cx: &mut TestAppContext,
    ) -> Option<String> {
        let id = id.into();
        self.frame(cx, |window, _| {
            window
                .try_find(id)
                .and_then(|element| element.label().map(str::to_owned))
        })
    }

    pub(crate) fn selected(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| {
            window.try_find(id).and_then(|element| element.selected()) == Some(true)
        })
    }

    /// The labels of `name`'s rows, `(name, 0)` onwards, until one is
    /// absent.
    pub(crate) fn rows(&self, name: &'static str, cx: &mut TestAppContext) -> Vec<String> {
        (0..)
            .map_while(|index| self.label((name, index as usize), cx))
            .collect()
    }
}

/// A root as the repository list gives it.
pub(crate) fn key(root: &Path) -> String {
    root.to_str().unwrap().to_owned()
}
