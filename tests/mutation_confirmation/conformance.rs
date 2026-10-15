//! The conformance table, for the commands that need confirmation; see
//! `support::conformance`.
//!
//! Each row's request presents the confirmation of a preview of it. The
//! request that is refused presents a confirmation no preview gave, and
//! the one with the wrong consent presents none.

use super::{
    support::{
        conformance::{Ask, Home as Target, Row, run},
        mutation::drive_confirmed,
    },
    *,
};

/// An operation ID no journal holds.
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZZ";

const REFUSED: &str = "confirmation_not_found";

const ROWS: [Row; 6] = [
    Row::new("repo create", true, REFUSED, |rule| {
        let desk = Desk::new();
        let site = Site::new();
        drive_confirmed(
            &desk.service,
            desk.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => site.create(),
                Ask::Changed => Mutation::RepoCreate(RepoCreateInput {
                    slug_prefix: Some("zz".to_owned()),
                    ..site.input()
                }),
            },
            // The target appears, with a file in it.
            &|| {
                fs::create_dir(&site.root).unwrap();
                fs::write(site.root.join("notes.txt"), "mine\n").unwrap();
            },
            rule,
        );
        Ok(())
    }),
    Row::new("repo enable", true, REFUSED, |rule| {
        let desk = Desk::new();
        let plain = Plain::new();
        drive_confirmed(
            &desk.service,
            desk.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => plain.enable(),
                Ask::Changed => Mutation::RepoEnable(RepoEnableInput {
                    slug_prefix: Some("zz".to_owned()),
                    ..plain.input()
                }),
            },
            // The worktree becomes dirty.
            &|| fs::write(plain.root.join("untracked.txt"), "new\n").unwrap(),
            rule,
        );
        Ok(())
    }),
    Row::new("remote remove", true, REFUSED, |rule| {
        let home = Home::new();
        home.add_remote("backup", BACKUP);
        drive_confirmed(
            home.service(),
            home.world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => home.remove_remote("backup"),
                Ask::Changed => home.remove_remote("another"),
            },
            // The remote is selected for publication.
            &|| home.select(REQUEST_3, Some("backup")),
            rule,
        );
        Ok(())
    }),
    Row::new("repo remove", true, REFUSED, |rule| {
        let home = Home::new();
        drive_confirmed(
            home.service(),
            home.world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => home.remove_repository(),
                // The command has no input but its repository: another
                // command.
                Ask::Changed => home.remove_remote("backup"),
            },
            // Another removes the registration.
            &|| home.remove_registration_by_hand(),
            rule,
        );
        Ok(())
    }),
    Row::new("key delete", true, REFUSED, |rule| {
        let keyring = Keyring::new();
        drive_confirmed(
            keyring.service(),
            keyring.world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => keyring.delete_key(),
                Ask::Changed => keyring.delete(SharedKeyId::new()),
            },
            // The key is selected.
            &|| {
                keyring.service().select_shared_key(keyring.id).unwrap();
            },
            rule,
        );
        Ok(())
    }),
    Row::new("operation abandon", true, REFUSED, |rule| {
        let stranded = abandon::Stranded::new();
        drive_confirmed(
            &stranded.world.service,
            stranded.world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => stranded.abandon(),
                Ask::Changed => abandon::abandon(&stranded.world.root, UNKNOWN),
            },
            // The operation records another step.
            &|| {
                items::index(stranded.world.data.path())
                    .execute(
                        "UPDATE operation_records
                            SET completed_step = 'authoring_checkpoint_observed'
                          WHERE state != 'completed'",
                        [],
                    )
                    .unwrap();
            },
            rule,
        );
        Ok(())
    }),
];

#[test]
fn every_command_of_this_target_keeps_the_rules_of_the_boundary() {
    run(
        Target::Confirmation,
        &manyhands::repository::request_store::bound_commands(),
        &ROWS,
    );
}
