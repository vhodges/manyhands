//! A main window over a fixture, for the views' tests: frames, clicks,
//! keys and the labels a test reads.

use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{
    App, ElementId, Entity, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext,
    Window, WindowHandle, component::Root, px, size, test::TestWindowExt,
};

use super::workspace::Workspace;
use crate::desktop::{
    keys,
    model::operations::Operations,
    open_test_window_over,
    services::requests::{DesktopRequest, DesktopRequestId},
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

/// How long a test waits on the worker: long enough on a loaded machine,
/// inside the scheduler's fifteen seconds of parking.
pub(crate) const WAIT: Duration = Duration::from_secs(10);

/// How often a wait looks again while the worker runs.
const POLL: Duration = Duration::from_millis(10);

/// Waits, with the worker running, until `done` holds of `entity`, then
/// lets the reads that started finish. Panics after `WAIT` of real time.
/// Every test that waits on the worker waits here, never with
/// `VisualTestContext::condition`, whose timer gives up after three
/// seconds of parked time. The test allows parking.
pub(crate) async fn wait_until<T: 'static>(
    cx: &TestAppContext,
    entity: &Entity<T>,
    done: impl Fn(&T, &App) -> bool,
) {
    let deadline = Instant::now() + WAIT;
    loop {
        cx.run_until_parked();
        if entity.read_with(cx, |value, cx| done(value, cx)) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the awaited state did not arrive in {WAIT:?}"
        );
        cx.executor().timer(POLL).await;
    }
}

pub(crate) struct Shell {
    pub window: WindowHandle<Root>,
    pub workspace: Entity<Workspace>,
    pub backend: Arc<DelayingBackend>,
    pub fixture: Fixture,
}

impl Shell {
    /// Opens the main window over `fixture`, whose repositories are
    /// already made, and lets its startup reads finish. Nothing is
    /// refreshed at launch, so a test that does not wait for the worker
    /// need not.
    pub(crate) fn open(fixture: Fixture, cx: &mut TestAppContext) -> Self {
        Self::start(fixture, false, |_| {}, cx)
    }

    /// Opens the main window as the application does, refreshing every
    /// repository at launch. `before` runs on the backend before the
    /// window opens, to hold what the test needs held. The test waits for
    /// the worker, so it allows parking.
    pub(crate) fn launch(
        fixture: Fixture,
        before: impl FnOnce(&DelayingBackend),
        cx: &mut TestAppContext,
    ) -> Self {
        Self::start(fixture, true, before, cx)
    }

    fn start(
        fixture: Fixture,
        launch_refresh: bool,
        before: impl FnOnce(&DelayingBackend),
        cx: &mut TestAppContext,
    ) -> Self {
        cx.update(gpui_kit::init);
        cx.update(keys::bind);
        // Dialogs open without their entrance animation, so a frame finds
        // them where they rest.
        cx.update(|cx| cx.set_reduce_motion(true));
        let backend = fixture.delaying_backend();
        before(&backend);
        let window = cx.update(|cx| open_test_window_over(backend.clone(), launch_refresh, cx));
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
        // Tall enough that an item, its later actions and a few expanded
        // entries of the operation area are all on screen, where a click
        // reaches them.
        shell.visual(cx).simulate_resize(size(px(1440.), px(1400.)));
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

    /// Presses and releases `key`, as a keyboard does. `keys` only presses:
    /// GPUI clicks a focused element on Enter or Space when the key is
    /// released.
    pub(crate) fn press(&self, key: &str, cx: &mut TestAppContext) {
        let keystroke = Keystroke::parse(key).unwrap();
        let mut visual = self.visual(cx);
        visual.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        visual.simulate_event(KeyUpEvent { keystroke });
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

    /// Whether the element `id` is a button that does not say it takes
    /// input. The Kit's button reports no disabled flag (its snapshot's
    /// `disabled()` is `None` either way), so a test pairs this with a
    /// click that changes nothing.
    pub(crate) fn inert_button(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| {
            window.try_find(id).is_some_and(|button| {
                button.role() == Some(gpui_kit::Role::Button) && button.disabled() != Some(false)
            })
        })
    }

    /// Whether the element `id` is drawn.
    pub(crate) fn present(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| window.try_find(id).is_some())
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

    /// Selects the repository at `root` and lets its reads finish.
    pub(crate) fn select(&self, root: &Path, cx: &mut TestAppContext) {
        self.workspace.update(cx, |workspace, cx| {
            workspace.select_repository(key(root), cx)
        });
        self.run(cx);
    }

    /// The window's request model.
    pub(crate) fn operations(&self, cx: &mut TestAppContext) -> Entity<Operations> {
        self.workspace
            .read_with(cx, |workspace, _| workspace.services().operations().clone())
    }

    /// The requests of this session that `pick` picks, newest first.
    pub(crate) fn requests(
        &self,
        cx: &mut TestAppContext,
        pick: impl Fn(&DesktopRequest) -> bool,
    ) -> Vec<DesktopRequestId> {
        let operations = self.operations(cx);
        operations.read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter(|request| pick(request))
                .map(|request| request.id)
                .collect()
        })
    }

    /// Waits, with the worker running, until `done` holds of the request
    /// model, then lets the reads that started finish. The test allows
    /// parking.
    pub(crate) async fn until(
        &self,
        cx: &mut TestAppContext,
        done: impl Fn(&Operations) -> bool + 'static,
    ) {
        let operations = self.operations(cx);
        wait_until(cx, &operations, move |operations, _| done(operations)).await;
    }

    /// Waits until the request `id` has ended: finished, removed or not
    /// sent.
    pub(crate) async fn until_ended(&self, id: DesktopRequestId, cx: &mut TestAppContext) {
        use crate::desktop::services::requests::RequestState;
        self.until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            })
        })
        .await;
    }
}

/// A root as the repository list gives it.
pub(crate) fn key(root: &Path) -> String {
    root.to_str().unwrap().to_owned()
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
