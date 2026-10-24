//! The Manyhands desktop application: startup and its first window.
//!
//! Every window's first entity is GPUI Kit's `Root`; the window's view is
//! created inside `Root`'s own `cx.new`, so its entity comes after.

mod keys;
mod model;
mod services;
mod view;

use std::{rc::Rc, sync::Arc};

use gpui_kit::{
    App, AppContext, Bounds, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, TestSupportExt, Window, WindowBounds, WindowHandle,
    WindowOptions,
    component::{
        Root,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px, size,
};
use manyhands::{
    drafts::DraftStore,
    repository::{RepositoryError, RepositoryErrorKind, RepositoryService},
};

use keys::Quit;
use services::{LibraryBackend, Services};
use view::workspace::Workspace;

/// Opens the service the desktop runs over. Production opens the default
/// data directory; tests open a temporary one.
pub(crate) type Opener = Rc<dyn Fn() -> Result<RepositoryService, RepositoryError>>;

pub(crate) const STARTUP_FAILURE_DATA_DIRECTORY: &str =
    "Manyhands could not open its data directory.";
pub(crate) const STARTUP_FAILURE_INDEX: &str = "Manyhands could not open its repository index.";
pub(crate) const STARTUP_FAILURE_OTHER: &str = "Manyhands could not start.";

/// Runs the desktop application. The Git transport is initialized before
/// this is called.
pub(crate) fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            keys::bind(cx);
            let opener: Opener = Rc::new(RepositoryService::open_default);
            cx.spawn(async move |cx| {
                cx.update(|cx| start(opener, cx));
            })
            .detach();
        });
}

/// Opens the service and its main window, or, when the service cannot
/// open, the startup failure window.
fn start(opener: Opener, cx: &mut App) -> WindowHandle<Root> {
    match opener() {
        Ok(service) => {
            let services = library_services(service, cx);
            open_main_window(services, cx)
        }
        Err(error) => open_startup_failure_window(StartupFailure::from(&error), opener, cx),
    }
}

fn window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1280.), px(840.)),
            cx,
        ))),
        ..Default::default()
    }
}

/// Builds `Root` over a new workspace. `Root`'s entity is reserved before
/// this runs, so the workspace's comes after it.
fn root_over_workspace(services: Services, window: &mut Window, cx: &mut Context<Root>) -> Root {
    let workspace = cx.new(|cx| Workspace::new(services, window, cx));
    Root::new(workspace, window, cx)
}

/// The production services: the library and the draft store in the
/// default data directory, with no delays, and its worker. A store that
/// cannot open leaves the desktop running without drafts.
fn library_services(service: RepositoryService, cx: &mut App) -> Services {
    let drafts = DraftStore::open_default();
    Services::start(Arc::new(LibraryBackend::new(service, drafts)), cx)
}

fn open_main_window(services: Services, cx: &mut App) -> WindowHandle<Root> {
    cx.open_window(window_options(cx), |window, cx| {
        cx.new(|cx| root_over_workspace(services, window, cx))
    })
    .expect("failed to open the Manyhands window")
}

fn open_startup_failure_window(
    failure: StartupFailure,
    opener: Opener,
    cx: &mut App,
) -> WindowHandle<Root> {
    cx.open_window(window_options(cx), |window, cx| {
        cx.new(|cx| {
            let view = cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                StartupFailureView {
                    failure,
                    opener,
                    opened: false,
                    focus,
                    quit_requested: false,
                }
            });
            Root::new(view, window, cx)
        })
    })
    .expect("failed to open the Manyhands window")
}

/// Why the service did not open: the error's kind only, never its text.
#[derive(Clone, Copy)]
pub(crate) struct StartupFailure {
    kind: RepositoryErrorKind,
}

impl From<&RepositoryError> for StartupFailure {
    fn from(error: &RepositoryError) -> Self {
        Self { kind: error.kind }
    }
}

impl StartupFailure {
    /// The fixed message for the failure's kind.
    fn message(self) -> &'static str {
        match self.kind {
            RepositoryErrorKind::Io => STARTUP_FAILURE_DATA_DIRECTORY,
            RepositoryErrorKind::Sqlite => STARTUP_FAILURE_INDEX,
            _ => STARTUP_FAILURE_OTHER,
        }
    }
}

/// The only view when the service cannot open: the message, Retry and
/// Quit. With no service, nothing else is offered.
pub(crate) struct StartupFailureView {
    failure: StartupFailure,
    opener: Opener,
    /// The service opened and the workspace is about to replace this
    /// view: a second Retry would open it, and start a worker, again.
    opened: bool,
    /// Focused when the window opens, so Quit reaches this view.
    focus: FocusHandle,
    #[cfg_attr(not(test), allow(dead_code))] // Read by tests only.
    quit_requested: bool,
}

impl StartupFailureView {
    #[cfg(test)]
    pub(crate) fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// With no service there is no worker to wait for: quit at once.
    fn quit(&mut self, cx: &mut Context<Self>) {
        self.quit_requested = true;
        cx.quit();
    }

    #[cfg(test)]
    pub(crate) fn message(&self) -> &'static str {
        self.failure.message()
    }

    /// Opens again; on success the window's Root is replaced by one over
    /// the workspace, created before the workspace as at startup.
    fn retry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.opened {
            return;
        }
        match (self.opener)() {
            Ok(service) => {
                self.opened = true;
                let services = library_services(service, cx);
                window.defer(cx, move |window, cx| {
                    window.replace_root(cx, |window, cx| root_over_workspace(services, window, cx));
                })
            }
            Err(error) => {
                self.failure = StartupFailure::from(&error);
                cx.notify();
            }
        }
    }
}

impl Render for StartupFailureView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("startup-failure")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Quit, _, cx| this.quit(cx)))
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .id("startup-failure-message")
                    .test_support()
                    .child(self.failure.message()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div().id("startup-retry").test_support().child(
                            Button::new("startup-retry-button")
                                .primary()
                                .label("Retry")
                                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.retry(window, cx)
                                })),
                        ),
                    )
                    .child(
                        div().id("startup-quit").test_support().child(
                            Button::new("startup-quit-button")
                                .label("Quit")
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.quit(cx))),
                        ),
                    ),
            )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[derive(Default)]
struct TestFixtures(Vec<test_support::fixtures::Fixture>);

#[cfg(all(test, feature = "desktop-test-support"))]
impl gpui_kit::Global for TestFixtures {}

/// Opens the main window over a new fixture's delaying backend, which
/// shares the fixture's own service. The fixture lives as long as the
/// test's application.
#[cfg(all(test, feature = "desktop-test-support"))]
fn open_test_window(cx: &mut App) -> WindowHandle<Root> {
    let fixture = test_support::fixtures::Fixture::new();
    let (window, _) = open_test_window_with(&fixture, cx);
    cx.default_global::<TestFixtures>().0.push(fixture);
    window
}

/// Opens the main window over a delaying backend that shares `fixture`'s
/// service, and gives the backend so a test can hold its reads and jobs.
/// The test keeps the fixture. Nothing is refreshed at launch: such a
/// test does not wait for the worker.
#[cfg(all(test, feature = "desktop-test-support"))]
fn open_test_window_with(
    fixture: &test_support::fixtures::Fixture,
    cx: &mut App,
) -> (
    WindowHandle<Root>,
    Arc<test_support::delaying::DelayingBackend>,
) {
    let backend = fixture.delaying_backend();
    (open_test_window_over(backend.clone(), false, cx), backend)
}

/// Opens the main window over `backend`, refreshing every repository at
/// launch when `launch_refresh` is set.
#[cfg(all(test, feature = "desktop-test-support"))]
fn open_test_window_over(
    backend: Arc<test_support::delaying::DelayingBackend>,
    launch_refresh: bool,
    cx: &mut App,
) -> WindowHandle<Root> {
    let services = Services::start(backend, cx);
    let services = if launch_refresh {
        services
    } else {
        services.without_launch_refresh()
    };
    open_main_window(services, cx)
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod mod_tests;
