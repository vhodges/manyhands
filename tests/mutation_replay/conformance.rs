//! The conformance table, for the commands that need neither a
//! confirmation nor a server; see `support::conformance`. Each module
//! of this target that drives a family of commands gives the rows of
//! that family, from its own fixtures and requests.

use crate::support::conformance::{Home, Row, run};

#[test]
fn every_command_of_this_target_keeps_the_rules_of_the_boundary() {
    let rows: Vec<Row> = super::bindings::CONFORMANCE
        .into_iter()
        .chain(super::remotes::CONFORMANCE)
        .chain(super::index::CONFORMANCE)
        .chain(super::key_generation::CONFORMANCE)
        .chain(super::keys::CONFORMANCE)
        .chain(super::resume::CONFORMANCE)
        .chain(super::local_sync::CONFORMANCE)
        .collect();
    run(
        Home::Replay,
        &manyhands::repository::request_store::bound_commands(),
        &rows,
    );
}
