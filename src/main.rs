#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod desktop;
mod startup_reporting;

fn main() {
    // Hook registration creates no threads and must cover Git startup panics.
    startup_reporting::install_panic_hook();
    // SAFETY: this is the first runtime initialization, before GPUI or any
    // other startup code can create background threads or use Git concurrently.
    if unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.is_err() {
        startup_reporting::report_startup_failure();
        std::process::exit(1);
    }
    desktop::run();
}
