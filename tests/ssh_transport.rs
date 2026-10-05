#[allow(dead_code, unused_imports)]
#[path = "../src/lib.rs"]
mod production;
pub use production::*;
#[path = "ssh_transport/formats.rs"]
mod formats;
#[path = "ssh_transport/session.rs"]
mod session;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;
#[path = "ssh_transport/state.rs"]
mod state;
#[path = "ssh_transport/transfer.rs"]
mod transfer;

fn main() {
    // SAFETY: actual main, before runner, fixture, watchdog, or runtime threads.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH transport initialization failed");
        std::process::exit(1);
    }
    let cases: Vec<_> = session::CASES
        .iter()
        .chain(transfer::CASES)
        .chain(state::CASES)
        .chain(formats::CASES)
        .copied()
        .collect();
    ssh_harness::run(&cases);
}
