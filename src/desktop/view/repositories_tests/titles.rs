//! The journey of `titles_of_what_is_not_shared` (Cycle 03a, Part C, R-9):
//! four real repositories with polling on, every poll answered by the
//! double, and the titles each row's tooltip names of what is not shared.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::repository::{LeaseKind, RepositoryService};

use super::{super::rows, support::*};
use crate::desktop::{
    model::{
        polling::{AttemptTarget, Disposition, TargetKind},
        repository_state::NOT_SHARED_YET,
    },
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::{
        harness::{Shell, key},
        polling_window::PollingWindow,
    },
};

/// An item ID of repository group `group`, number `n`.
fn tid(group: u8, n: u8) -> String {
    format!("01ARZ3NDEKTSV4RRFFQ69G5T{group}{n}")
}

fn document(id: &str) -> AttemptTarget {
    AttemptTarget {
        kind: Some(TargetKind::Document),
        ..item(id)
    }
}

/// An item a local conflict resolution added: it has an ID and no kind.
fn unkinded(id: &str) -> AttemptTarget {
    AttemptTarget {
        kind: None,
        disposition: Disposition::LocalAhead,
        ..item(id)
    }
}

/// What the tooltip of the row named `name` says first.
fn says(shell: &Shell, name: &str, cx: &mut TestAppContext) -> String {
    shell.workspace.read_with(cx, |workspace, cx| {
        let row = rows(workspace, cx)
            .into_iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("no row {name}"));
        row.tooltip[0].clone()
    })
}

/// What a row says, as its first sentence names `names`.
fn names(names: &str) -> String {
    format!("{NOT_SHARED_YET}{names}.")
}

/// Lets what is ready run, then the busy retries' timers, then what that
/// made ready.
fn settle(shell: &Shell, cx: &mut TestAppContext) {
    shell.run(cx);
    cx.executor().advance_clock(Duration::from_secs(1));
    shell.run(cx);
}

pub(super) async fn journey(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (a, b, c, d) = (
        fixture.create_repository("a"),
        fixture.create_repository("b"),
        fixture.create_repository("c"),
        fixture.create_repository("d"),
    );
    fixture.create_ticket(&a, &tid(1, 1), "Alpha", "open");
    fixture.create_document(&a, &tid(1, 2), "docs/guide.md", "Guide");
    fixture.create_ticket(&b, &tid(2, 1), "Beta", "open");
    fixture.create_ticket(&c, &tid(3, 1), "Gamma", "open");
    for n in 1..=7 {
        fixture.create_ticket(&d, &tid(4, n), &format!("S{n}"), "open");
    }
    let roots = [&a, &b, &c, &d];
    for root in roots {
        fixture.publish_over_ssh(root);
        fixture.refresh(root);
    }
    let data = fixture.data_dir().to_owned();
    let (window, ()) = PollingWindow::over(
        fixture,
        &[],
        |backend, clock| {
            for _ in 0..roots.len() {
                backend.answer_poll(completed(clock));
            }
        },
        cx,
    );
    let PollingWindow {
        shell,
        services,
        clock,
        ..
    } = window;
    let services = services.with_polling();
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    let keys = roots.map(|root| key(root));
    shell
        .until_drawn(cx, "every launch poll", |_, cx| {
            services.scheduler().read_with(cx, |scheduler, _| {
                keys.iter().all(|root| {
                    scheduler
                        .schedule()
                        .repo(root)
                        .is_some_and(|repo| repo.last.is_some() && !repo.in_flight())
                })
            })
        })
        .await;
    let read = |kind: ReadKind| reads(&shell, kind);
    let [key_a, key_b, key_c, key_d] = &keys;

    // A title the loaded lists give is named at once, with no read: `a` is
    // the chosen repository, so its Tickets and Documents lists are loaded.
    shell.select(&a, cx);
    shell
        .until_drawn(cx, "a's lists", |shell, cx| {
            shell.workspace.read_with(cx, |workspace, _| {
                workspace.tickets().list().rows().is_some()
                    && workspace.documents().list().rows().is_some()
            })
        })
        .await;
    checked(
        &services,
        &clock,
        key_a,
        vec![item(&tid(1, 1)), document(&tid(1, 2))],
        cx,
    );
    shell.run(cx);
    assert!(says(&shell, "a", cx).starts_with(&names("Alpha, Guide")));
    assert_eq!(read(ReadKind::ShareTitles), 0, "named from the lists");

    // An item in another repository is read: the job is held at its first
    // call. Notifies meanwhile start no second job and use no try. Once
    // released the title is read, and named; the item with no kind is
    // named by its ID and never read; main is not an item to read.
    let held = shell.backend.hold_read(ReadKind::ShareTitles);
    checked(
        &services,
        &clock,
        key_b,
        vec![main_current(), item(&tid(2, 1)), unkinded(&tid(2, 2))],
        cx,
    );
    shell.run(cx);
    assert_eq!(read(ReadKind::ShareTitles), 1, "one job, held");
    assert!(says(&shell, "b", cx).starts_with(&names(&format!("{}, {}", tid(2, 1), tid(2, 2)))));
    for _ in 0..4 {
        notify(&services, cx);
        shell.run(cx);
    }
    assert_eq!(read(ReadKind::ShareTitles), 1, "no second job while held");
    let _ = held.send(());
    shell.run(cx);
    let after_b = read(ReadKind::ShareTitles);
    assert_eq!(after_b, 2, "resolved once, read the one item with a kind");
    assert!(says(&shell, "b", cx).starts_with(&names(&format!("Beta, {}", tid(2, 2)))));
    notify(&services, cx);
    shell.run(cx);
    assert_eq!(read(ReadKind::ShareTitles), after_b, "found is done");

    // A read that stays busy is read again at each notify, and after three
    // tries the item is named by its ID and not read again. The notifies
    // while the first read is held used no try.
    let held = shell.backend.hold_read(ReadKind::ShareTitles);
    let lease =
        RepositoryService::hold_lease_for_testing(&c, &data, LeaseKind::CacheWrite).unwrap();
    checked(&services, &clock, key_c, vec![item(&tid(3, 1))], cx);
    shell.run(cx);
    let before = read(ReadKind::ShareTitles);
    assert_eq!(before, after_b + 1, "one job, held");
    for _ in 0..4 {
        notify(&services, cx);
        shell.run(cx);
    }
    assert_eq!(
        read(ReadKind::ShareTitles),
        before,
        "no second job while held"
    );
    let _ = held.send(());
    settle(&shell, cx);
    let per_job = read(ReadKind::ShareTitles) - before + 1;
    assert_eq!(per_job, 1, "the lease makes the resolve itself busy");
    let tries = |cx: &mut TestAppContext| {
        notify(&services, cx);
        settle(&shell, cx);
    };
    tries(cx);
    tries(cx);
    assert_eq!(
        read(ReadKind::ShareTitles),
        before - 1 + 3 * per_job,
        "three tries, one job each"
    );
    assert!(says(&shell, "c", cx).starts_with(&names(&tid(3, 1))));
    drop(lease);
    tries(cx);
    tries(cx);
    assert_eq!(
        read(ReadKind::ShareTitles),
        before - 1 + 3 * per_job,
        "not read again after three tries, though it could be now"
    );
    assert!(says(&shell, "c", cx).starts_with(&names(&tid(3, 1))));

    // Seven items not shared: five are named, and only those are read.
    let before = read(ReadKind::ShareTitles);
    checked(
        &services,
        &clock,
        key_d,
        (1..=7).map(|n| item(&tid(4, n))).collect(),
        cx,
    );
    shell.run(cx);
    assert_eq!(read(ReadKind::ShareTitles), before + 1 + 5, "five read");
    assert!(
        says(&shell, "d", cx)
            .starts_with(&format!("{NOT_SHARED_YET}S1, S2, S3, S4, S5 and 2 more."))
    );
    notify(&services, cx);
    shell.run(cx);
    assert_eq!(read(ReadKind::ShareTitles), before + 6, "all five done");
}
