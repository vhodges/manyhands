//! The whole-repository refresh (the fast-writes design, "The
//! whole-repository refresh"): what `index refresh`, a poll's hand-off and the
//! desktop's start run. Its first step, with no lease held, observes only the
//! contexts that changed since they were stored, that are dirty or new, or
//! that hold a transient problem. Its second, under the lease, checks each of
//! them again by stat and stores those that still match.

use std::{cell::Cell, collections::BTreeMap, fs, rc::Rc};

use git2::Repository;

use super::{
    activity::take_diffed,
    command_index_tests::{
        Fixture, OTHER, SPARE, TICKET, commit, item, ticket_path, ticket_source, write,
    },
    discovery::{
        clear_scan_hook, listed_context, observe_context, set_context_scan_hook, set_scan_hook,
    },
    git_fixtures::active_worktree_at,
    *,
};
use crate::results::ResultCode;

/// A whole refresh of the fixture's repository under `operation_id`: the
/// ID a refresh that must be tried again is retried under.
fn refresh_as(f: &Fixture, operation_id: OperationId) -> RefreshOutcome {
    f.service
        .refresh_repository(RefreshRepositoryRequest {
            root: f.root.clone(),
            operation_id,
        })
        .unwrap()
}

/// A whole refresh under `operation_id` that completes.
fn refreshed_as(f: &Fixture, operation_id: OperationId) {
    let outcome = refresh_as(f, operation_id);
    assert!(
        matches!(outcome, RefreshOutcome::Refreshed { .. }),
        "{outcome:?}"
    );
}

/// A whole refresh, under an ID of its own, that completes.
fn refreshed(f: &Fixture) {
    refreshed_as(f, OperationId::new());
}

/// Edits the ticket `id` in its own context, outside Manyhands.
fn edit(f: &Fixture, id: &str, title: &str) {
    write(
        &f.worktree(id),
        &ticket_path(id),
        &ticket_source(id, title, ""),
    );
}

/// The title the index holds for `id` in its own context, and that
/// context's mark; `None` while nothing is stored for it.
fn own_copy(f: &Fixture, id: &str) -> Option<(String, i64)> {
    f.index()
        .query_row(
            "SELECT discovered_items.title, contexts.dirty
               FROM discovered_items JOIN contexts ON contexts.id = discovered_items.context_id
              WHERE contexts.item_id = ?1 AND discovered_items.item_id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .unwrap()
}

/// Whether anything is stored for the context of `id`.
fn stored_context(f: &Fixture, id: &str) -> bool {
    f.index()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM contexts WHERE item_id = ?1)",
            [id],
            |row| row.get(0),
        )
        .unwrap()
}

/// Asserts that the contexts `watched` did not observe kept their rows and
/// basis as they were stored.
fn assert_unobserved_unchanged(case: &str, watched: &command_index_tests::Watched) {
    for (context, before) in &watched.before {
        if !watched.observed.contains(context.as_str()) && watched.after.contains_key(context) {
            assert_eq!(
                watched.after.get(context),
                Some(before),
                "{case}: {context}'s rows and basis changed"
            );
        }
    }
}

/// What the refresh observes, by what changed since everything was stored,
/// and how often: each context that changed once, and no other. The others
/// keep their rows, basis and indexed time, and the index reads current.
#[test]
fn a_refresh_observes_only_changed_contexts() {
    /// What changed, how it changes, and the contexts the refresh observes.
    type Case = (&'static str, fn(&Fixture), &'static [&'static str]);
    let cases: [Case; 8] = [
        ("nothing", |_| {}, &[]),
        (
            "an edit of TICKET's file",
            |f| edit(f, TICKET, "Edited by hand"),
            &["TICKET"],
        ),
        (
            "a commit on the primary",
            |f| {
                write(&f.root, "fixture.txt", "changed\n");
                commit(&Repository::open(&f.root).unwrap(), &["fixture.txt"]);
            },
            &["primary"],
        ),
        (
            "OTHER marked dirty",
            |f| {
                f.index()
                    .execute("UPDATE contexts SET dirty = 1 WHERE item_id = ?1", [OTHER])
                    .unwrap();
            },
            &["OTHER"],
        ),
        (
            "OTHER stored by an older build, with no basis",
            |f| {
                f.index()
                    .execute(
                        "UPDATE contexts SET basis = NULL, indexed_at = NULL WHERE item_id = ?1",
                        [OTHER],
                    )
                    .unwrap();
            },
            &["OTHER"],
        ),
        (
            "a transient problem in OTHER",
            |f| {
                f.index()
                    .execute(
                        "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
                         SELECT repository_id, id, 'notes.md', 'source',
                                'the source cannot be inspected: permission denied', 1
                           FROM contexts WHERE item_id = ?1",
                        [OTHER],
                    )
                    .unwrap();
            },
            &["OTHER"],
        ),
        (
            "a permanent problem in OTHER",
            |f| {
                f.index()
                    .execute(
                        "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
                         SELECT repository_id, id, 'notes.md', 'missing-front-matter',
                                'add front matter', 1
                           FROM contexts WHERE item_id = ?1",
                        [OTHER],
                    )
                    .unwrap();
            },
            &[],
        ),
        (
            "SPARE's context, which the index stores nothing for",
            |f| {
                let prepared = f
                    .service
                    .prepare_context(AuthoringTarget {
                        root: f.root.clone(),
                        kind: AuthoringKind::Ticket,
                        item_id: item(SPARE),
                        intent: ContextIntent::Edit,
                        operation_id: OperationId::new(),
                    })
                    .unwrap();
                assert!(matches!(prepared, ContextProvisionOutcome::Created(_)));
                f.index()
                    .execute("DELETE FROM contexts WHERE item_id = ?1", [SPARE])
                    .unwrap();
            },
            &["SPARE"],
        ),
    ];
    for (case, change, expected) in cases {
        let f = Fixture::new();
        change(&f);
        let watched = f.watch(|| refreshed(&f));
        let once = expected
            .iter()
            .map(|name| (*name, 1))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(watched.counts, once, "{case}");
        assert_unobserved_unchanged(case, &watched);
        assert_eq!(f.status(), IndexStatusState::Current, "{case}");
    }
}

/// A refresh of a repository nothing changed in diffs no commit: it observes
/// no context, so neither the primary's activity map nor any item walk runs.
#[test]
fn a_refresh_of_an_unchanged_repository_diffs_no_commits() {
    let f = Fixture::new();
    take_diffed();
    let watched = f.watch(|| refreshed(&f));
    assert_eq!(take_diffed(), 0);
    assert!(watched.observed.is_empty(), "{:?}", watched.observed);
    assert_eq!(f.status(), IndexStatusState::Current);
}

/// After `k` commits on the primary (outside Manyhands), a refresh observes
/// the primary alone and diffs exactly those `k` commits, advancing the
/// primary's activity map over them; no item context's history is walked.
#[test]
fn a_refresh_after_k_new_primary_commits_diffs_k() {
    for k in [1, 3] {
        let f = Fixture::new();
        let repository = Repository::open(&f.root).unwrap();
        for commit_number in 0..k {
            write(&f.root, "fixture.txt", &format!("change {commit_number}\n"));
            commit(&repository, &["fixture.txt"]);
        }
        take_diffed();
        let watched = f.watch(|| refreshed(&f));
        assert_eq!(take_diffed(), k, "{k} commits");
        assert_eq!(
            watched.observed.iter().copied().collect::<Vec<_>>(),
            ["primary"],
            "{k} commits"
        );
        assert_eq!(f.status(), IndexStatusState::Current, "{k} commits");
    }
}

/// A context the first step observed that an outside `git` command changes
/// before the second step (here a commit in its worktree, between the steps)
/// is not stored: it keeps what was stored for it, marked dirty, and
/// `index refresh` answers `busy`, as a race does. Nothing else is observed.
/// The same request, sent again, observes that context alone and stores it.
#[test]
fn an_outside_change_between_the_refresh_steps_leaves_the_context_dirty_and_answers_busy() {
    let f = Fixture::new();
    edit(&f, TICKET, "Edited by hand");
    let worktree = f.worktree(TICKET);
    f.service.set_observation_hook_for_testing(move || {
        let repository = Repository::open(&worktree).unwrap();
        commit(&repository, &[&ticket_path(TICKET)]);
    });
    let request = RequestId::new();
    let refresh = || {
        Mutation::IndexRefresh(IndexRefreshInput {
            root: f.root.clone(),
        })
    };

    let watched = f.watch(|| {
        let raced = f.execute_as(request, refresh());
        assert_eq!(raced.code, ResultCode::Busy, "{raced:?}");
    });
    assert_eq!(watched.counts, BTreeMap::from([("TICKET", 1)]));
    assert_unobserved_unchanged("raced", &watched);
    assert_eq!(own_copy(&f, TICKET), Some(("Ticket".to_owned(), 1)));
    assert_eq!(f.status(), IndexStatusState::Stale);

    let watched = f.watch(|| {
        let retried = f.execute_as(request, refresh());
        assert_eq!(retried.code, ResultCode::Ok, "{retried:?}");
    });
    assert_eq!(watched.counts, BTreeMap::from([("TICKET", 1)]));
    assert_eq!(own_copy(&f, TICKET), Some(("Edited by hand".to_owned(), 0)));
    assert_eq!(f.status(), IndexStatusState::Current);
}

/// A context that appears between the two steps (a worktree made outside
/// Manyhands) was not observed, so nothing is stored for it: no row, which
/// Table A would present. The refresh answers `RetryRequired`, and its retry
/// observes and stores it.
#[test]
fn a_context_that_appeared_between_the_refresh_steps_is_not_stored() {
    let f = Fixture::new();
    let root = f.root.clone();
    f.service.set_observation_hook_for_testing(move || {
        let repository = Repository::open(&root).unwrap();
        active_worktree_at(&repository, &root, "ticket", SPARE, SPARE);
    });

    let operation_id = OperationId::new();
    let outcome = refresh_as(&f, operation_id);
    assert!(
        matches!(&outcome, RefreshOutcome::RetryRequired { context: Some(context), .. }
            if *context == f.worktree(SPARE)),
        "{outcome:?}"
    );
    assert!(!stored_context(&f, SPARE));
    assert_eq!(f.status(), IndexStatusState::Stale);

    let watched = f.watch(|| refreshed_as(&f, operation_id));
    assert_eq!(watched.counts, BTreeMap::from([("SPARE", 1)]));
    assert_eq!(own_copy(&f, SPARE), Some(("Spare".to_owned(), 0)));
    assert_eq!(f.status(), IndexStatusState::Current);
}

/// A store of a context that lands between the two steps (another call
/// indexed it, from the context as it is now) is not undone: the second step
/// finds the context's row changed since the first and holding what is on
/// disk, and leaves it as it is, unmarked. The refresh completes.
#[test]
fn a_store_that_landed_after_step_one_is_not_marked_dirty() {
    let f = Fixture::new();
    edit(&f, TICKET, "Edited by hand");
    let (root, worktree, registry) = (
        f.root.clone(),
        f.worktree(TICKET),
        f.data.path().join(REGISTRY_FILE),
    );
    f.service.set_observation_hook_for_testing(move || {
        write(
            &worktree,
            &ticket_path(TICKET),
            &ticket_source(TICKET, "Landed", ""),
        );
        let repository = Repository::open(&root).unwrap();
        let listed = listed_context(&root, &ContextKey::Item(item(TICKET))).unwrap();
        let observation = observe_context(
            &repository,
            &root,
            &listed,
            &mut PrimaryActivityMap::default(),
        );
        let mut index = rusqlite::Connection::open(&registry).unwrap();
        let transaction = index.transaction().unwrap();
        let repository_id: i64 = transaction
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        persist_context_index(&transaction, repository_id, &observation).unwrap();
        transaction.commit().unwrap();
    });

    refreshed(&f);
    assert_eq!(own_copy(&f, TICKET), Some(("Landed".to_owned(), 0)));
    assert_eq!(f.status(), IndexStatusState::Current);
}

/// No observation runs under the lease: each context, observed because
/// every one is dirty, is read while the repository's lease is free, and the
/// first step's scan of the repository begins with it free as well.
#[test]
fn a_refresh_runs_no_observation_under_the_lease() {
    let f = Fixture::new();
    f.index()
        .execute("UPDATE contexts SET dirty = 1", [])
        .unwrap();
    let reads = Rc::new(Cell::new(0));
    let unleased = |data: std::path::PathBuf, root: std::path::PathBuf| {
        let reads = reads.clone();
        move || {
            assert!(leases_are_free(&data, &root), "observed under the lease");
            reads.set(reads.get() + 1);
        }
    };
    let contexts = [f.root.clone(), f.worktree(TICKET), f.worktree(OTHER)];
    for worktree in &contexts {
        set_context_scan_hook(
            worktree.clone(),
            unleased(f.data.path().to_owned(), f.root.clone()),
        );
    }
    set_scan_hook(
        f.root.clone(),
        unleased(f.data.path().to_owned(), f.root.clone()),
    );

    refreshed(&f);
    for worktree in &contexts {
        clear_scan_hook(worktree);
    }
    // The scan, then each of the three contexts once.
    assert_eq!(reads.get(), 4);
    assert_eq!(f.status(), IndexStatusState::Current);
}

/// A worktree removed outside Manyhands is no longer listed: the refresh
/// deletes its context's rows, observes nothing else, and the index reads
/// current.
#[test]
fn a_refresh_reconciles_a_removed_worktree() {
    let f = Fixture::new();
    fs::remove_dir_all(f.worktree(OTHER)).unwrap();

    let watched = f.watch(|| refreshed(&f));
    assert!(watched.observed.is_empty(), "{:?}", watched.observed);
    assert!(watched.before.contains_key("OTHER"));
    assert!(!watched.after.contains_key("OTHER"));
    assert!(!stored_context(&f, OTHER));
    assert_unobserved_unchanged("removed", &watched);
    assert_eq!(f.status(), IndexStatusState::Current);
}

/// The first refresh after the migration that gave each context its basis
/// (every row dirty, with no basis, indexed time or activity base) observes
/// every context exactly once and stores it. The next observes none.
#[test]
fn the_first_refresh_after_migration_observes_every_context_once() {
    let f = Fixture::new();
    f.index()
        .execute(
            "UPDATE contexts SET dirty = 1, basis = NULL, indexed_at = NULL, activity_base = NULL",
            [],
        )
        .unwrap();

    let watched = f.watch(|| refreshed(&f));
    assert_eq!(
        watched.counts,
        BTreeMap::from([("OTHER", 1), ("TICKET", 1), ("primary", 1)])
    );
    assert_eq!(f.status(), IndexStatusState::Current);
    for (context, (basis, indexed_at, dirty, _)) in &watched.after {
        assert!(basis.is_some() && indexed_at.is_some(), "{context}");
        assert_eq!(*dirty, 0, "{context}");
    }

    let watched = f.watch(|| refreshed(&f));
    assert!(watched.observed.is_empty(), "{:?}", watched.observed);
    assert_eq!(f.status(), IndexStatusState::Current);
}
