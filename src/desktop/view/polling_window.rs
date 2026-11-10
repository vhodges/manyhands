//! The window over the polling double (D6 Task 13), shared by the privacy
//! sweep and the keyboard journey: the main window, the recording double
//! under its services, polling switched on, a clock in the test's hand, and
//! the repositories a fixture made. A poll's answer is the double's
//! (`answer_poll`, `fail_poll`), a retirement's or a Check now's is the
//! delaying backend's (`answer_job`), so nothing reaches a transport.

use std::{path::PathBuf, sync::Arc};

use gpui_kit::{TestAppContext, px, size};
use manyhands::repository::OperationId;
use time::OffsetDateTime;

use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, Clock, Disposition, PollAttempt, Reason, TargetKind, TestClock,
    },
    services::Services,
    test_support::{fixtures::Fixture, polling::PollingBackend},
    view::harness::{Shell, key},
};

pub(crate) struct PollingWindow {
    pub shell: Shell,
    pub services: Services,
    pub backend: Arc<PollingBackend>,
    pub clock: Arc<TestClock>,
    pub roots: Vec<String>,
}

impl PollingWindow {
    /// The window over `fixture`, whose repositories `roots` the caller made
    /// (and published) in it, at the default window size, polling off until
    /// [`Self::start`]: a poll that started with the window would be
    /// preempted by the first job the test submits. What the polls are to
    /// answer is set up by `before`, which runs over the double before
    /// anything is open, with the clock the scheduler will read (an answer
    /// carries its own completion time, and one older than the launch is
    /// the previous process's, L-1), and gives what it made (a hold on a
    /// job, say).
    pub(crate) fn over<T>(
        fixture: Fixture,
        roots: &[PathBuf],
        before: impl FnOnce(&PollingBackend, &TestClock) -> T,
        cx: &mut TestAppContext,
    ) -> (Self, T) {
        cx.executor().allow_parking();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let clock = Arc::new(TestClock::at(OffsetDateTime::now_utc()));
        let made = before(&backend, &clock);
        let inner = backend.inner_shared();
        let shell = Shell::over_double(fixture, backend.clone(), inner, cx);
        let services = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().clone());
        let services = cx.update(|cx| services.with_clock(clock.clone(), cx));
        shell.visual(cx).simulate_resize(size(px(1280.), px(840.)));
        shell.run(cx);
        (
            Self {
                shell,
                services,
                backend,
                clock,
                roots: roots.iter().map(|root| key(root)).collect(),
            },
            made,
        )
    }

    /// Polling is switched on and the repositories are listed: the
    /// scheduler reads them and polls the due ones.
    pub(crate) fn start(&self, cx: &mut TestAppContext) {
        let services = self.services.clone().with_polling();
        let roots = self.roots.clone();
        cx.update(|cx| services.repositories_listed(&roots, cx));
        self.shell.run(cx);
    }

    /// The scheduler's attention entries as `(reason, roots)`.
    pub(crate) fn entries(&self, cx: &mut TestAppContext) -> Vec<(Reason, Vec<String>)> {
        self.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .attention()
                .into_iter()
                .map(|entry| (entry.reason, entry.roots))
                .collect()
        })
    }
}

/// A completed attempt of `root` that reported `targets`, as the schedule's
/// own transition takes it (the loop is not started: the items are offered
/// Retire…).
pub(crate) fn reported(
    shell: &Shell,
    services: &Services,
    clock: &TestClock,
    root: &str,
    targets: Vec<(manyhands::canonical::ItemId, TargetKind)>,
    cx: &mut TestAppContext,
) {
    let now = clock.now();
    let attempt = PollAttempt {
        root: root.to_owned(),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: targets
            .into_iter()
            .map(|(item, kind)| AttemptTarget {
                item_id: Some(item),
                kind: Some(kind),
                disposition: match kind {
                    TargetKind::Ticket => Disposition::Closed,
                    TargetKind::Document => Disposition::Promoted,
                },
            })
            .collect(),
        index_pending: false,
    };
    services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    shell.run(cx);
}
