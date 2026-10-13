//! The conformance table, for the bridges; see `support::conformance`.
//!
//! `repo identity-set` needs confirmation: its request presents the
//! confirmation of a preview of it. The request that is refused presents
//! a confirmation no preview gave, and the one with the wrong consent
//! presents none. A refusal by the binding itself, for a preview that is
//! stale, is in `identity::boundary`.
//!
//! `document repair` and `ticket repair` need confirmation too, and are
//! driven the same way: the request that is changed carries another
//! source, which is another ID. A refusal by the binding itself, for a
//! preview that is stale, is in `tests/mutation_confirmation/repair.rs`.
//!
//! `folder create` needs none. The request that is refused is one the
//! domain refuses, for a file at its path: it was accepted first, and
//! the rule is that nothing of it is left.

use manyhands::repository::{FolderCreateInput, InitialsChange, Mutation, RepoIdentitySetInput};

use super::support::{
    confirmation::{Desk, Plain},
    conformance::{Ask, Home as Target, Row, run},
    git_home, items,
    mutation::{World, drive_confirmed, drive_plain},
    repair::{self, Shelf},
};

const ROWS: [Row; 4] = [
    Row::new("document repair", true, "confirmation_not_found", |rule| {
        git_home::isolated();
        let shelf = Shelf::new();
        let token = shelf.path_token(repair::MARKER_PATH);
        let with = |id: &str| {
            shelf.document_repair_with(repair::MARKER_PATH, &token, &repair::document(id))
        };
        drive_confirmed(
            &shelf.service,
            shelf.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => with(repair::ADOPTED),
                Ask::Changed => with(repair::UNHELD),
            },
            rule,
        );
        Ok(())
    }),
    Row::new("ticket repair", true, "confirmation_not_found", |rule| {
        git_home::isolated();
        let shelf = Shelf::new();
        let path = items::ticket_path(repair::TICKET_MARKER);
        let with = |title: &str| {
            let source = repair::ticket(repair::TICKET_MARKER, &format!("team: {title}\n"));
            shelf.ticket_repair(&path, &source)
        };
        drive_confirmed(
            &shelf.service,
            shelf.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => with("one"),
                Ask::Changed => with("other"),
            },
            rule,
        );
        Ok(())
    }),
    Row::new("folder create", false, "occupied_path", |rule| {
        git_home::isolated();
        let world = World::new();
        let create = |path: &str| {
            Mutation::FolderCreate(FolderCreateInput {
                root: world.root.clone(),
                path: path.to_owned(),
            })
        };
        drive_plain(
            &world.service,
            world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Unconsented => create("docs/notes"),
                Ask::Changed => create("docs/other"),
                // The file `docs/guide.md`.
                Ask::Refused => create("docs/guide.md"),
            },
            rule,
        );
        Ok(())
    }),
    Row::new(
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
    ),
];

#[test]
fn every_command_of_this_target_keeps_the_rules_of_the_boundary() {
    run(
        Target::Bridges,
        &manyhands::repository::request_store::bound_commands(),
        &ROWS,
    );
}
