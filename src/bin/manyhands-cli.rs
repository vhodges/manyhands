fn main() {
    // SAFETY: this is the first action and no threads have been spawned.
    if unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.is_err() {
        eprintln!("Git transport initialization failed");
        std::process::exit(1);
    }
}
