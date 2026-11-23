//! The quit tests' rig (D6 Task 10): the main window over the polling
//! double, polling switched on, the scheduler's clock in the test's hand.
//! The GPUI clock, which the ten-second timer reads, is moved with
//! `cx.executor().advance_clock`; nothing sleeps for it.

use std::{path::PathBuf, sync::Arc, time::Duration};

use gpui_kit::TestAppContext;
use manyhands::repository::PollCompletion;
use time::OffsetDateTime;

use super::QuitPhase;
use crate::desktop::{
    model::polling::{Clock, TestClock},
    services::{
        Services,
        poll_convert::{Library, ReportParts},
        worker::WorkerHandle,
    },
    test_support::{
        delaying::JobGate,
        fixtures::Fixture,
        polling::{CancelStep, PollingBackend},
    },
    view::harness::{Shell, key},
};

pub(crate) struct Rig {
    pub shell: Shell,
    pub services: Services,
    pub backend: Arc<PollingBackend>,
    pub clock: Arc<TestClock>,
    pub roots: Vec<String>,
}

impl Rig {
    /// Repositories `names`, each published over SSH, in a window that
    /// allows parking: the worker thread answers what the test waits for.
    pub(crate) fn new(names: &[&str], cx: &mut TestAppContext) -> Self {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let roots: Vec<PathBuf> = names
            .iter()
            .map(|name| fixture.create_repository(name))
            .collect();
        Self::over(fixture, &roots, cx)
    }

    /// As `new`, over `fixture` and the repositories the caller made in it,
    /// which are published over SSH here; parking is the caller's to allow.
    pub(crate) fn over(
        fixture: Fixture,
        repositories: &[PathBuf],
        cx: &mut TestAppContext,
    ) -> Self {
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let mut roots = Vec::new();
        for root in repositories {
            fixture.publish_over_ssh(root);
            fixture.refresh(root);
            roots.push(key(root));
        }
        let inner = backend.inner_shared();
        let shell = Shell::over_double(fixture, backend.clone(), inner, cx);
        let clock = Arc::new(TestClock::at(OffsetDateTime::now_utc()));
        let services = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().clone());
        let services = cx.update(|cx| services.with_clock(clock.clone(), cx).with_polling());
        Self {
            shell,
            services,
            backend,
            clock,
            roots,
        }
    }

    /// The repositories are listed: the launch poll of each due one.
    pub(crate) fn list(&self, cx: &mut TestAppContext) {
        let services = self.services.clone();
        let roots = self.roots.clone();
        cx.update(|cx| services.repositories_listed(&roots, cx));
        self.shell.run(cx);
    }

    /// A completed poll's answer.
    pub(crate) fn completed(&self) -> Library {
        let now = self.clock.utc();
        Library::Polled {
            parts: ReportParts {
                completion: PollCompletion::Completed,
                completed_at: now,
                next_eligible_at: Some(now + time::Duration::seconds(300)),
                targets: Vec::new(),
                outcome: None,
            },
            index_pending: false,
        }
    }

    /// A cancelled poll's answer, as the library reports one.
    pub(crate) fn cancelled(&self) -> Library {
        let now = self.clock.utc();
        Library::Polled {
            parts: ReportParts {
                completion: PollCompletion::Cancelled,
                completed_at: now,
                next_eligible_at: None,
                targets: Vec::new(),
                outcome: None,
            },
            index_pending: false,
        }
    }

    pub(crate) fn worker(&self) -> WorkerHandle {
        self.services.worker().clone()
    }

    /// The workspace's quit request, as the Quit action and the window's
    /// close request make it: whether the window may close now.
    pub(crate) fn request_quit(&self, cx: &mut TestAppContext) -> bool {
        let quit = self.shell.frame(cx, |window, cx| {
            self.shell
                .workspace
                .update(cx, |workspace, cx| workspace.request_quit(window, cx))
        });
        self.shell.run(cx);
        quit
    }

    pub(crate) fn phase(&self, cx: &mut TestAppContext) -> QuitPhase {
        self.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase())
    }

    /// What quitting says it waits for.
    pub(crate) fn text(&self, cx: &mut TestAppContext) -> Option<String> {
        self.shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.finishing_text(cx))
    }

    /// How many polls reached the library.
    pub(crate) fn polls(&self) -> usize {
        self.backend.polled_roots().len()
    }

    pub(crate) fn tick(&self, cx: &mut TestAppContext) {
        self.services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.tick(cx));
    }

    /// Whether the scheduler's timer is armed.
    pub(crate) fn armed(&self, cx: &mut TestAppContext) -> bool {
        self.services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.armed_for().is_some())
    }

    /// Moves the scheduler's clock.
    pub(crate) fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    /// Lets everything that can run now run, and the worker answer what it
    /// has, without waiting for anything new.
    pub(crate) async fn quiet(cx: &mut TestAppContext) {
        for _ in 0..20 {
            cx.run_until_parked();
            cx.executor().timer(Duration::from_millis(10)).await;
        }
        cx.run_until_parked();
    }

    /// Quitting is asked and not cancelled: the timer is gone, and however
    /// long the clock runs and the scheduler is woken, no poll reaches the
    /// library (R-33). `polls` is how many had before.
    pub(crate) async fn assert_frozen(&self, polls: usize, cx: &mut TestAppContext) {
        assert!(!self.armed(cx), "no timer waits for the next poll");
        self.advance(Duration::from_secs(301));
        self.tick(cx);
        Self::quiet(cx).await;
        assert_eq!(self.polls(), polls, "no poll while quitting is asked");
    }

    /// `action` cancels quitting: the poll that fell due meanwhile goes.
    pub(crate) async fn assert_resumed(&self, polls: usize, cx: &mut TestAppContext) {
        self.shell
            .until_drawn(cx, "the poll to resume", |_, _| self.polls() == polls + 1)
            .await;
    }

    /// Waits until the poll of `root` has been applied: its repository
    /// has an attempt on record.
    pub(crate) async fn until_applied(&self, root: &str, cx: &mut TestAppContext) {
        self.shell
            .until_drawn(cx, "the poll's answer", |_, cx| {
                self.services.scheduler().read_with(cx, |scheduler, _| {
                    scheduler
                        .schedule()
                        .repo(&root.to_owned())
                        .is_some_and(|repo| repo.last.is_some())
                })
            })
            .await;
    }
}

/// A launch poll of the rig's repository, held before its library call, and
/// a cancel the library's answers never make effective (a blocked connect,
/// K-1). The poll ends only when the gate is released, and then answers
/// `Cancelled`.
pub(crate) async fn held_poll(rig: &Rig, cx: &mut TestAppContext) -> JobGate {
    rig.backend
        .script_cancels(std::iter::repeat_n(CancelStep::Ineffective, 1000));
    rig.backend.answer_poll(rig.cancelled());
    let gate = rig.backend.hold_poll();
    rig.list(cx);
    rig.shell
        .until_drawn(cx, "the poll to start", |_, _| rig.polls() == 1)
        .await;
    gate.wait_reached();
    gate
}
