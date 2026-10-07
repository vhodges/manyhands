//! Opt-in scratch-only native demo. No repository services or document saves.
#[path = "editor_feasibility/adapter.rs"]
mod adapter;
#[path = "editor_feasibility/catalog.rs"]
mod catalog;
// S1 also exposes independent comparators/replacement policy used by its tests.
#[allow(dead_code)]
#[path = "editor_feasibility/evidence.rs"]
mod evidence;
#[path = "editor_feasibility/host.rs"]
mod host;
#[path = "editor_feasibility/observation.rs"]
mod observation;
#[allow(dead_code)]
#[path = "editor_feasibility/session.rs"]
mod session;

use gpui_kit::component::Root;
use gpui_kit::*;

fn main() {
    // SAFETY: first action, before platform/background worker initialization.
    if unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.is_err() {
        eprintln!("Git transport initialization failed");
        std::process::exit(1);
    }
    let mut capture_initial = false;
    let mut diagnostics = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--capture-initial" if !capture_initial => capture_initial = true,
            "--diagnostics" if !diagnostics => diagnostics = true,
            _ => {
                eprintln!("Usage: editor_feasibility [--capture-initial] [--diagnostics]");
                std::process::exit(2);
            }
        }
    }
    let documents = catalog::load().expect("fixed repository catalog must be readable");
    let provenance = host::running_provenance().expect("HEAD/lock provenance unavailable");
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            zorite_editor::bind_keys(cx);
            host::bind_keys(cx);
            let bounds = Bounds::centered(None, size(px(1280.), px(900.)), cx);
            cx.spawn(async move |cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        ..Default::default()
                    },
                    |window, cx| {
                        // cx.new reserves Root's entity before running this closure.
                        // Every child (including Host/editors) is allocated inside it.
                        cx.new(|cx| {
                            let host = cx.new(|cx| {
                                host::Host::new(
                                    documents,
                                    provenance,
                                    capture_initial,
                                    diagnostics,
                                    window,
                                    cx,
                                )
                            });
                            Root::new(host, window, cx)
                        })
                    },
                )
                .expect("failed to open scratch editor window");
            })
            .detach();
        });
}
