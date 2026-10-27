//! The conformance table, for the bridges; see `support::conformance`.
//!
//! `repo identity-set` needs confirmation: its request presents the
//! confirmation of a preview of it. The request that is refused presents
//! a confirmation no preview gave, and the one with the wrong consent
//! presents none. The preview is made stale by a value changed in the
//! repository's own configuration.
//!
//! `document repair` and `ticket repair` need confirmation too, and are
//! driven the same way: the request that is changed carries another
//! source, which is another ID. The preview is made stale by an edit of
//! the file, committed on primary.
//!
//! `context prepare` needs none, and is refused an edit of an item nothing holds.
//!
//! `folder create` needs none. The request that is refused is one the
//! domain refuses, for a file at its path: it was accepted first, and
//! the rule is that nothing of it is left.

use manyhands::repository::{
    AuthoringKind, ContextPrepareInput, ContextPrepareIntent, FolderCreateInput, InitialsChange,
    Mutation, RepoIdentitySetInput, TicketRepairInput,
};

use super::support::{
    confirmation::{Desk, Plain},
    conformance::{Ask, Home as Target, Row, run},
    git_home, items,
    mutation::{TICKET_NEW, World, drive_confirmed, drive_plain},
    repair::{self, Shelf},
};

/// What someone commits on primary over the file a preview was of.
const EDITED: &str = "---\nmanyhands_managed: true\n---\nEdited since the preview.\n";

const ROWS: [Row; 5] = [
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
            // The file is edited and committed on primary.
            &|| {
                shelf.commit_on_primary(repair::MARKER_PATH, EDITED);
            },
            rule,
        );
        Ok(())
    }),
    Row::new("ticket repair", true, "confirmation_not_found", |rule| {
        git_home::isolated();
        let shelf = Shelf::new();
        let path = items::ticket_path(repair::TICKET_MARKER);
        // The token the file has when the row begins: a request that
        // read it anew after the file changed would be another request
        // than the one its confirmation is of.
        let token = shelf.path_token(&path);
        let with = |title: &str| {
            Mutation::TicketRepair(TicketRepairInput {
                root: shelf.root.clone(),
                path: path.clone().into(),
                observation: token.clone(),
                source: repair::ticket(repair::TICKET_MARKER, &format!("team: {title}\n")),
            })
        };
        drive_confirmed(
            &shelf.service,
            shelf.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => with("one"),
                Ask::Changed => with("other"),
            },
            &|| {
                shelf.commit_on_primary(&path, EDITED);
            },
            rule,
        );
        Ok(())
    }),
    Row::new("context prepare", false, "item_not_found", |rule| {
        git_home::isolated();
        let world = World::new();
        let prepare = |id: &str, intent| {
            Mutation::ContextPrepare(ContextPrepareInput {
                root: world.root.clone(),
                kind: AuthoringKind::Ticket,
                id: items::item_id(id),
                intent,
            })
        };
        drive_plain(
            &world.service,
            world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Unconsented => {
                    prepare(items::TICKET_A, ContextPrepareIntent::Edit)
                }
                // The same item, for another purpose.
                Ask::Changed => prepare(items::TICKET_A, ContextPrepareIntent::Create),
                // A ticket no repository holds.
                Ask::Refused => prepare(TICKET_NEW, ContextPrepareIntent::Edit),
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
                // A value of the repository's own configuration changes.
                &|| {
                    git2::Config::open(&plain.root.join(".git/config"))
                        .unwrap()
                        .set_str("user.email", "later@example.invalid")
                        .unwrap();
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
