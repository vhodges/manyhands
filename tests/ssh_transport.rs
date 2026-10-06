#[allow(dead_code, unused_imports)]
#[path = "../src/lib.rs"]
mod production;
pub use production::*;
#[path = "ssh_transport/endpoints.rs"]
mod endpoints;
#[path = "ssh_transport/failures.rs"]
mod failures;
#[path = "ssh_transport/formats.rs"]
mod formats;
#[path = "ssh_transport/privacy.rs"]
mod privacy;
#[path = "ssh_transport/session.rs"]
mod session;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
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
    let cases: Vec<_> = [
        (
            "runner_options_regression",
            ssh_harness::runner_options_regression as fn() -> _,
        ),
        (
            "capture_limit_regression",
            ssh_harness::capture_limit_regression as fn() -> _,
        ),
        (
            "raw_output_regression",
            ssh_harness::raw_output_regression as fn() -> _,
        ),
    ]
    .iter()
    .chain(endpoints::CASES)
    .chain(session::CASES)
    .chain(transfer::CASES)
    .chain(failures::CASES)
    .chain(privacy::CASES)
    .chain(state::CASES)
    .chain(formats::CASES)
    .copied()
    .collect();
    ssh_harness::run(&cases);
}
