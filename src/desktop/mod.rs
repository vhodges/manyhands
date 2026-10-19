//! The Manyhands desktop application: startup and its first window.
//!
//! Every window's first entity is GPUI Kit's `Root`; the window's view is
//! created inside `Root`'s own `cx.new`, so its entity comes after.

mod keys;
// Until the views that read through these arrive (Task 4), only tests
// use most of them.
#[cfg_attr(not(test), allow(dead_code, unused_imports))]
mod model;
#[cfg_attr(not(test), allow(dead_code, unused_imports))]
mod services;
mod view;

use std::{rc::Rc, sync::Arc};

use gpui_kit::{
    App, AppContext, Bounds, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, TestSupportExt, Window, WindowBounds, WindowHandle, WindowOptions,
    component::{
        Root,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px, size,
};
use manyhands::repository::{RepositoryError, RepositoryErrorKind, RepositoryService};

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
        Ok(service) => open_main_window(library_services(service), cx),
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
    let workspace = cx.new(|_| Workspace::new(services));
    Root::new(workspace, window, cx)
}

/// The production services: the library, with no delays.
fn library_services(service: RepositoryService) -> Services {
    Services::new(Arc::new(LibraryBackend::new(service)))
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
            let view = cx.new(|_| StartupFailureView { failure, opener });
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
}

impl StartupFailureView {
    #[cfg(test)]
    pub(crate) fn message(&self) -> &'static str {
        self.failure.message()
    }

    /// Opens again; on success the window's Root is replaced by one over
    /// the workspace, created before the workspace as at startup.
    fn retry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match (self.opener)() {
            Ok(service) => {
                let services = library_services(service);
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
                                .on_click(|_, _, cx| cx.quit()),
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
    let services = Services::new(fixture.delaying_backend());
    cx.default_global::<TestFixtures>().0.push(fixture);
    open_main_window(services, cx)
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod mod_tests;
