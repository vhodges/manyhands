#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod desktop;

fn main() {
    // SAFETY: this is the first action, before GPUI creates background threads.
    if unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.is_err() {
        eprintln!("Git transport initialization failed");
        std::process::exit(1);
    }
    desktop::run();
}
