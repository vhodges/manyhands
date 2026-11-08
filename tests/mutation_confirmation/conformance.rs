//! The conformance table, for the commands that need confirmation; see
//! `support::conformance`.
//!
//! Each row's request presents the confirmation of a preview of it. The
//! request that is refused presents a confirmation no preview gave, and
//! the one with the wrong consent presents none.

use manyhands::repository::{
    AuthoringKind, ItemResolveHistoryInput, RemoteResolveHistoryInput, RemoteSelectInput,
    ResolveHistoryChoice,
};

use super::{
    support::{
        conformance::{Ask, Home as Target, Row, run},
        mutation::drive_confirmed,
    },
    *,
};

/// An operation ID no journal holds.
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZZ";

/// A second ticket, with a context of its own.
const OTHER_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FCM";

const REFUSED: &str = "confirmation_not_found";

const ROWS: [Row; 8] = [
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
    Row::new("item resolve-history", true, REFUSED, |rule| {
        let lost = CacheLoss::new(true);
        drive_confirmed(
            &lost.world.service,
            lost.world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => lost.resolve_item(),
                // Another item, which has a branch of its own to resolve, so the
                // refusal is the mismatch and not a missing item.
                Ask::Changed => lost.resolve_other_item(),
            },
            // A newer fetch moves the generation the preview showed.
            &|| lost.move_generation(),
            rule,
        );
        Ok(())
    }),
    Row::new("remote resolve-history", true, REFUSED, |rule| {
        let lost = CacheLoss::new(false);
        drive_confirmed(
            &lost.world.service,
            lost.world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => lost.clear(),
                // The command has no input but its repository: another
                // command.
                Ask::Changed => lost.deselect(),
            },
            &|| lost.move_generation(),
            rule,
        );
        Ok(())
    }),
];

/// A repository with a publication remote whose cache was lost, which
/// holds a ticket context when `with_ticket`.
struct CacheLoss {
    world: World,
}

impl CacheLoss {
    fn new(with_ticket: bool) -> Self {
        let world = World::new();
        git2::Repository::open(&world.root)
            .unwrap()
            .remote("origin", "git@example.invalid:team/project.git")
            .unwrap();
        let selected = world.execute(REQUEST_1, Self::select(&world, Some("origin")));
        assert_eq!(selected.outcome, Outcome::Success);
        if with_ticket {
            for (request, id) in [(REQUEST_2, mutation::TICKET_NEW), (REQUEST_3, OTHER_TICKET)] {
                let created = world.execute(request, world.create(id));
                assert_eq!(created.outcome, Outcome::Success, "{}", created.message);
            }
        }
        let lost = Self { world };
        lost.registry()
            .execute("UPDATE remote_polling_state SET history_unknown=1", [])
            .unwrap();
        lost
    }

    fn registry(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(
            self.world
                .data
                .path()
                .join(manyhands::repository::REGISTRY_FILE),
        )
        .unwrap()
    }

    fn select(world: &World, name: Option<&str>) -> Mutation {
        Mutation::RemoteSelect(RemoteSelectInput {
            root: world.root.clone(),
            name: name.map(str::to_owned),
        })
    }

    fn resolve(&self, id: &str) -> Mutation {
        Mutation::ItemResolveHistory(ItemResolveHistoryInput {
            root: self.world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(id),
            choice: ResolveHistoryChoice::Publish,
        })
    }

    fn resolve_item(&self) -> Mutation {
        self.resolve(mutation::TICKET_NEW)
    }

    fn resolve_other_item(&self) -> Mutation {
        self.resolve(OTHER_TICKET)
    }

    fn clear(&self) -> Mutation {
        Mutation::RemoteResolveHistory(RemoteResolveHistoryInput {
            root: self.world.root.clone(),
        })
    }

    fn deselect(&self) -> Mutation {
        Self::select(&self.world, None)
    }

    fn move_generation(&self) {
        self.registry()
            .execute(
                "UPDATE remote_polling_state SET configuration_generation=configuration_generation+1",
                [],
            )
            .unwrap();
    }
}

#[test]
fn every_command_of_this_target_keeps_the_rules_of_the_boundary() {
    run(
        Target::Confirmation,
        &manyhands::repository::request_store::bound_commands(),
        &ROWS,
    );
}
