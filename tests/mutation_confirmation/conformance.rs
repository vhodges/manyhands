//! The conformance table, for the commands that need confirmation; see
//! `support::conformance`.
//!
//! Each row's request presents the confirmation of a preview of it. The
//! request that is refused presents a confirmation no preview gave, and
//! the one with the wrong consent presents none.

use manyhands::repository::{
    AuthoringKind, ItemResolveHistoryInput, RemoteResolveHistoryInput, RemoteSelectInput,
    ResolveHistoryChoice, TicketWithdrawClosureInput,
};

use super::{
    support::{
        conformance::{Ask, Expect, Home as Target, Row, run},
        mutation::drive_confirmed,
    },
    *,
};

/// An operation ID no journal holds.
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZZ";

/// A second ticket, with a context of its own.
const OTHER_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FCM";

const REFUSED: &str = "confirmation_not_found";

const ROWS: [Row; 10] = [
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
    Row::new("repo setup-cancel", true, REFUSED, |rule| {
        let desk = Desk::new();
        let plain = Plain::new();
        plain
            .fixture
            .repository
            .remote("origin", "ssh://git@example.invalid/team/repo.git")
            .unwrap();
        // A recorded setup: the enable stops before it connects, because
        // this process has not initialized the Git transport.
        let enable = || {
            Mutation::RepoEnable(RepoEnableInput {
                publication_remote: Some("origin".to_owned()),
                integration: Some(manyhands::canonical::Integration::PullRequest),
                ..plain.input()
            })
        };
        let prepared = serde_json::to_value(desk.service.prepare(&enable())).unwrap();
        let confirmation = prepared["data"]["confirmation_id"].as_str().unwrap();
        let recorded = desk.execute("01ARZ3NDEKTSV4RRFFQ69G5FW1", confirmation, enable());
        assert_eq!(recorded.code, manyhands::results::ResultCode::InternalError);
        let head = plain.head().to_string();
        drive_confirmed(
            &desk.service,
            desk.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => {
                    Mutation::RepoSetupCancel(manyhands::repository::RepoSetupCancelInput {
                        root: plain.root.clone(),
                    })
                }
                // The command has no input but its repository: another
                // command.
                Ask::Changed => Mutation::RepoRemove(manyhands::repository::RepoRemoveInput {
                    root: plain.root.clone(),
                }),
            },
            // The setup is published.
            &|| {
                items::index(desk.data.path())
                    .execute(
                        "UPDATE repository_setups
                            SET base_oid = ?1, commit_oid = ?1, published_oid = ?1",
                        [&head],
                    )
                    .unwrap();
            },
            rule,
        );
        Ok(())
    }),
    // A finished withdraw publishes its commit, which needs a server: its
    // same-request replay is pinned by the one SSH case of
    // `tests/mutation_remote` (design P14), so rules 1 and 2 are exempt here.
    // Its refusal, consent and stale-preview rules run over a withdraw that is
    // offered and never confirmed.
    Row {
        replay: Expect::Exempt(
            "a finished withdraw needs a server; its same-request replay is pinned by P14's case in tests/mutation_remote",
        ),
        mismatch: Expect::Exempt(
            "a finished withdraw needs a server; the rule finishes the request first, and P14's case pins the same request",
        ),
        ..Row::new("ticket withdraw-closure", true, REFUSED, |rule| {
            let offered = Withdrawable::new(true);
            drive_confirmed(
                &offered.world.service,
                offered.world.data.path(),
                &|ask| match ask {
                    Ask::Same | Ask::Refused | Ask::Unconsented => offered.withdraw(TICKET_A),
                    // Another ticket: the command's one input.
                    Ask::Changed => offered.withdraw(items::TICKET_B),
                },
                // The closure on the branch is another than the preview showed.
                &|| offered.reclose("2027-01-15T08:00:00Z"),
                rule,
            );
            Ok(())
        })
    },
];

/// A repository with a publication remote and a ticket whose context holds a
/// closure of its own, or none: a withdraw of it is offered, or has nothing
/// to withdraw. No request reaches the remote, which nothing listens at.
struct Withdrawable {
    world: World,
}

impl Withdrawable {
    fn new(closed: bool) -> Self {
        let world = World::new();
        git2::Repository::open(&world.root)
            .unwrap()
            .remote("origin", "git@example.invalid:team/project.git")
            .unwrap();
        let selected = world.execute(REQUEST_1, CacheLoss::select(&world, Some("origin")));
        assert_eq!(selected.outcome, Outcome::Success, "{}", selected.message);
        // The tracking primary, where primary is: a withdraw is blocked with
        // `primary_missing` without it, and nothing is then offered.
        let repository = git2::Repository::open(&world.root).unwrap();
        let head = repository.head().unwrap().target().unwrap();
        repository
            .reference("refs/remotes/origin/main", head, true, "fixture")
            .unwrap();
        // The ticket's editing context.
        let token = world.token(TICKET_A);
        let saved = world.execute(REQUEST_2, world.save(TICKET_A, &token));
        assert_eq!(saved.outcome, Outcome::Success, "{}", saved.message);
        let offered = Self { world };
        if closed {
            offered.reclose("2026-09-30T12:34:56Z");
        }
        offered
    }

    fn withdraw(&self, id: &str) -> Mutation {
        Mutation::TicketWithdrawClosure(TicketWithdrawClosureInput {
            root: self.world.root.clone(),
            id: items::item_id(id),
        })
    }

    /// Commits a closure of the ticket's own, closed at `closed_at`, in its
    /// editing context, over any it holds.
    fn reclose(&self, closed_at: &str) {
        let worktree = self.world.worktree(TICKET_A);
        let path = items::ticket_path(TICKET_A);
        let source = fs::read_to_string(worktree.join(&path)).unwrap();
        let open: String = source
            .lines()
            .filter(|line| !line.starts_with("closed_at: ") && !line.starts_with("closed_by: "))
            .map(|line| format!("{line}\n"))
            .collect();
        fs::write(
            worktree.join(&path),
            open.replacen(
                "---\n",
                &format!(
                    "---\nclosed_at: {closed_at}\nclosed_by: Ada Lovelace <ada@example.invalid>\n"
                ),
                1,
            ),
        )
        .unwrap();
        let repository = git2::Repository::open(&worktree).unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new(&path)).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        let signature =
            git2::Signature::now("Manyhands Test", "manyhands-test@example.invalid").unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "closed on the branch",
                &tree,
                &[&parent],
            )
            .unwrap();
        items::refresh_completely(&self.world.service, &self.world.root);
    }
}

/// Part E: a ticket whose context and tracking ref hold no closure has nothing
/// to withdraw. The prepare answers `already_applied` with no data and no
/// confirmation, and writes nothing (the golden of the contract).
#[test]
fn withdraw_with_no_closure_is_already_applied() {
    let none = Withdrawable::new(false);
    let rows = |world: &World| {
        items::index(world.data.path())
            .query_row("SELECT count(*) FROM confirmation_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
    };
    let before = rows(&none.world);
    let noop = none.world.service.prepare(&none.withdraw(TICKET_A));
    assert_eq!(noop.code, ResultCode::AlreadyApplied, "{}", noop.message);
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_eq!(noop.data, None);
    assert!(noop.recovery.is_empty());
    assert_eq!(rows(&none.world), before, "nothing to confirm");
    assert_golden(
        "ticket_withdraw_closure_already_applied",
        &none.world.root,
        &noop,
    );
}

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
        // A complete fetch that advertised nothing: absence is read from a
        // current batch.
        lost.registry()
            .execute("INSERT INTO remote_observation_batches(repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) SELECT repository_id,remote_name,primary_branch,configuration_generation,100,1 FROM remote_polling_state WHERE remote_name IS NOT NULL", [])
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
            integration: None,
            forge: None,
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
            .execute_batch(
                "UPDATE remote_polling_state SET configuration_generation=configuration_generation+1; UPDATE remote_observation_batches SET configuration_generation=configuration_generation+1",
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
