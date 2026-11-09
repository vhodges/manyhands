//! The Checking for updates page over a real service (D6 design Testing,
//! item 3): a paused repository is not eligible, the scheduler reads that
//! from the library, and the row says Paused. The remote is configured and
//! never contacted.

use std::{cell::Cell, rc::Rc};

use gpui_kit::TestAppContext;
use manyhands::repository::{AutomaticPollEligibility, PollIneligibility};

use super::polling_support::Rig;
use crate::desktop::services::JobKind;

/// The library says a paused repository is `NotEligible(Paused)`; the
/// scheduler, listing it with polling on, reads that policy and does not
/// poll it; and the page's row says Paused with the switch off. The
/// scheduler tells the page when it looks again, for the words that follow
/// the clock.
#[gpui_kit::test]
async fn settings_polling_real_paused(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    let root = rig.roots[0].clone();
    rig.shell.fixture.pause_polling(&root);

    let eligibility = rig
        .shell
        .fixture
        .service()
        .automatic_poll_eligibility(&root)
        .unwrap();
    assert!(matches!(
        eligibility,
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::Paused,
            ..
        }
    ));

    let services = rig.services.clone().with_polling();
    let roots = vec![rig.key(0)];
    cx.update(|cx| services.repositories_listed(&roots, cx));
    let scheduled = {
        let (services, key) = (services.clone(), rig.key(0));
        move |cx: &mut TestAppContext| {
            services.scheduler().read_with(cx, |scheduler, _| {
                scheduler
                    .schedule()
                    .repo(&key)
                    .is_some_and(|repo| repo.policy.paused)
            })
        }
    };
    let ready = scheduled.clone();
    rig.shell
        .until_drawn(cx, "the paused repository to be scheduled", move |_, cx| {
            ready(cx)
        })
        .await;

    rig.open(1, cx).await;
    assert_eq!(rig.status(0, cx).as_deref(), Some("Paused"));
    assert_eq!(
        rig.shell.checked(("polling-switch-toggle", 0usize), cx),
        Some(false)
    );
    // Paused is not polled: the worker ran no poll job.
    assert!(
        !rig.shell
            .backend
            .jobs_seen()
            .iter()
            .any(|kind| matches!(kind, JobKind::Poll | JobKind::PollOnce)),
        "a paused repository is not polled"
    );

    // A look at the clock tells the page, whose words follow it.
    let looked = Rc::new(Cell::new(0));
    let counted = looked.clone();
    let _watch = cx.update(|cx| {
        cx.observe(services.scheduler(), move |_, _| {
            counted.set(counted.get() + 1)
        })
    });
    services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.tick(cx));
    rig.shell.run(cx);
    assert!(looked.get() >= 1, "the tick told its observers");
}

/// A repository the scheduler has not read yet is told apart from one it
/// found no SSH publication remote for: the first reads "Reading the
/// remotes" (the schedule is not there yet), the second has no shared copy
/// (C4). Over a real service, with polling on and the list just shown.
#[gpui_kit::test]
async fn settings_polling_unplaced(cx: &mut TestAppContext) {
    use crate::desktop::model::polling::status::Unplaced;

    let rig = Rig::new(&[("a", true), ("b", false)], cx);
    let services = rig.services.clone().with_polling();
    // The third is no registration: its read fails.
    let missing = "/no/such/repository".to_owned();
    let roots = vec![rig.key(0), rig.key(1), missing.clone()];
    let unplaced = {
        let services = services.clone();
        move |root: String, cx: &mut TestAppContext| {
            services
                .scheduler()
                .read_with(cx, |scheduler, _| scheduler.unplaced(&root))
        }
    };
    // Not listed to the scheduler yet, and then listed with its remotes to
    // be read: either way it is not placed.
    assert_eq!(unplaced(rig.key(0), cx), Some(Unplaced::Reading));
    cx.update(|cx| services.repositories_listed(&roots, cx));
    for ix in 0..2 {
        assert_eq!(unplaced(rig.key(ix), cx), Some(Unplaced::Reading), "{ix}");
    }

    let (ready, key) = (services.clone(), rig.key(0));
    rig.shell
        .until_drawn(cx, "the reads to land", move |_, cx| {
            ready
                .scheduler()
                .read_with(cx, |scheduler, _| scheduler.schedule().repo(&key).is_some())
        })
        .await;
    // `a` is scheduled; `b` was read and has no publication remote; the
    // read of the third failed, and it says so.
    for ix in 0..2 {
        assert_eq!(unplaced(rig.key(ix), cx), None, "{ix}");
    }
    rig.shell
        .until_drawn(cx, "the failed read to land", {
            let (services, missing) = (services.clone(), missing.clone());
            move |_, cx| {
                services
                    .scheduler()
                    .read_with(cx, |scheduler, _| scheduler.unplaced(&missing))
                    == Some(Unplaced::Unreadable)
            }
        })
        .await;
    let scheduled = |root: String, cx: &mut TestAppContext| {
        services.scheduler().read_with(cx, |scheduler, _| {
            scheduler.schedule().repo(&root).is_some()
        })
    };
    assert!(scheduled(rig.key(0), cx));
    assert!(!scheduled(rig.key(1), cx));
    assert!(!scheduled(missing, cx));
}
