//! A world for the scheduler entity's tests (D6 Task 5): a fixture, the
//! recording double over its real service, `Services` with polling on and a
//! clock moved by hand. Nothing sleeps for the schedule: the clock is
//! injected. The tests wait only for the real worker thread.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::TestAppContext;
use manyhands::repository::PollCompletion;

use super::reconfigure::Heard;
use crate::desktop::{
    model::polling::{Clock, Phase, TestClock},
    services::{
        Services,
        poll_convert::{Library, ReportParts, TargetParts},
    },
    test_support::{WAIT, fixtures::Fixture, polling::PollingBackend},
};

pub(super) struct World {
    pub fixture: Fixture,
    pub backend: Arc<PollingBackend>,
    pub services: Services,
    pub clock: Arc<TestClock>,
}

impl World {
    /// Services over the double, the clock injected, no launch refresh;
    /// polling on when `polling`.
    pub fn new(polling: bool, cx: &mut TestAppContext) -> Self {
        Self::new_with(polling, cx, |_| {})
    }

    /// As [`Self::new`], with `before` run over the backend before the
    /// services start: a hold on the launch listing has to be in place
    /// before it is asked for.
    pub fn new_with(
        polling: bool,
        cx: &mut TestAppContext,
        before: impl FnOnce(&PollingBackend),
    ) -> Self {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        before(&backend);
        // Real time to start from: a real service stores real deadlines.
        let clock = Arc::new(TestClock::at(time::OffsetDateTime::now_utc()));
        let services = cx.update(|cx| {
            let services = Services::start(backend.clone(), cx)
                .without_launch_refresh()
                .with_clock(clock.clone(), cx);
            if polling {
                services.with_polling()
            } else {
                services
            }
        });
        Self {
            fixture,
            backend,
            services,
            clock,
        }
    }

    /// A repository publishing over SSH through `origin`.
    pub fn ssh_repo(&self, name: &str) -> String {
        let root = self.fixture.create_repository(name);
        self.fixture.publish_over_ssh(&root);
        root.to_str().unwrap().to_owned()
    }

    /// A repository with no remote.
    pub fn plain_repo(&self, name: &str) -> String {
        self.fixture
            .create_repository(name)
            .to_str()
            .unwrap()
            .to_owned()
    }

    /// A completed poll's answer, stored `interval` ahead of the clock.
    pub fn completed(&self) -> Library {
        self.report(PollCompletion::Completed, Vec::new(), false)
    }

    pub fn report(
        &self,
        completion: PollCompletion,
        targets: Vec<TargetParts>,
        index_pending: bool,
    ) -> Library {
        let now = self.clock.utc();
        Library::Polled {
            parts: ReportParts {
                completion,
                completed_at: now,
                next_eligible_at: Some(now + time::Duration::seconds(300)),
                targets,
                outcome: None,
            },
            index_pending,
        }
    }

    pub fn list(&self, roots: &[String], cx: &mut TestAppContext) {
        let services = self.services.clone();
        cx.update(|cx| services.repositories_listed(roots, cx));
    }

    pub fn activate(&self, cx: &mut TestAppContext) {
        let services = self.services.clone();
        cx.update(|cx| services.window_activated(cx));
    }

    pub fn heard(&self, heard: Vec<Heard>, cx: &mut TestAppContext) {
        self.services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.heard(heard, cx));
    }

    pub fn tick(&self, cx: &mut TestAppContext) {
        self.services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.tick(cx));
    }

    pub fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    pub fn phase(&self, root: &str, cx: &mut TestAppContext) -> Option<Phase> {
        self.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(&root.to_owned())
                .map(|repo| repo.phase.clone())
        })
    }

    pub fn scheduled(&self, root: &str, cx: &mut TestAppContext) -> bool {
        self.phase(root, cx).is_some()
    }

    /// Waits, letting the executor run, until `done` holds.
    pub async fn settle(
        cx: &mut TestAppContext,
        what: &str,
        mut done: impl FnMut(&mut TestAppContext) -> bool,
    ) {
        let deadline = Instant::now() + WAIT;
        loop {
            cx.run_until_parked();
            if done(cx) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            cx.executor().timer(Duration::from_millis(10)).await;
        }
    }

    /// Lets everything that can run now run, and the worker answer what it
    /// has, without waiting for anything new.
    pub async fn quiet(cx: &mut TestAppContext) {
        for _ in 0..20 {
            cx.run_until_parked();
            cx.executor().timer(Duration::from_millis(10)).await;
        }
        cx.run_until_parked();
    }
}
