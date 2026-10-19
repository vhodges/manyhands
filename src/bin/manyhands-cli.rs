// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

#[allow(dead_code)] // wired into main in Task 2
#[path = "manyhands-cli/catalog.rs"]
mod catalog;
#[allow(dead_code)] // wired into main in Task 2
#[path = "manyhands-cli/parse.rs"]
mod parse;

fn main() {
    // SAFETY: this is the first action and no threads have been spawned.
    if unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.is_err() {
        eprintln!("Git transport initialization failed");
        std::process::exit(1);
    }
}
