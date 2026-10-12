//! The conformance table, for the bridges; see `support::conformance`.
//!
//! `repo identity-set` needs confirmation: its request presents the
//! confirmation of a preview of it. The request that is refused presents
//! a confirmation no preview gave, and the one with the wrong consent
//! presents none. A refusal by the binding itself, for a preview that is
//! stale, is in `identity::boundary`.

use manyhands::repository::{InitialsChange, Mutation, RepoIdentitySetInput};

use super::support::{
    confirmation::{Desk, Plain},
    conformance::{Ask, Home as Target, Row, run},
    git_home,
    mutation::drive_confirmed,
};

const ROWS: [Row; 1] = [Row::new(
    "repo identity-set",
    true,
    "confirmation_not_found",
    |rule| {
        git_home::isolated();
        let desk = Desk::new();
        let plain = Plain::new();
        let set = |name: &str| {
            Mutation::RepoIdentitySet(RepoIdentitySetInput {
                root: plain.root.clone(),
                name: name.to_owned(),
                email: "ada@example.invalid".to_owned(),
                initials: InitialsChange::Unchanged,
            })
        };
        drive_confirmed(
            &desk.service,
            desk.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => set("Ada Example"),
                Ask::Changed => set("Ada Other"),
            },
            rule,
        );
        Ok(())
    },
)];

#[test]
fn every_command_of_this_target_keeps_the_rules_of_the_boundary() {
    run(
        Target::Bridges,
        &manyhands::repository::request_store::bound_commands(),
        &ROWS,
    );
}
