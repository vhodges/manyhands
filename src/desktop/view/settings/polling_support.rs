//! The Checking for updates page's test rig: the window over a fixture's
//! repositories, a clock moved by hand, and the schedule seeded through
//! the model's own transitions. What a control sends is read from the
//! requests the service received.

use std::{path::PathBuf, sync::Arc, time::Duration};

use gpui_kit::TestAppContext;
use manyhands::repository::OperationId;
use time::OffsetDateTime;

use crate::desktop::{
    model::polling::{Answer, Clock, Policy, PollAttempt, TestClock},
    services::{
        Services,
        requests::{DesktopRequest, Intent},
    },
    test_support::fixtures::Fixture,
    view::harness::{Shell, key},
};

/// The window, the services with a clock moved by hand, and the page's
/// repositories (named `a`, `b`, ... and published over SSH when `ssh`).
pub(crate) struct Rig {
    pub(crate) shell: Shell,
    pub(crate) services: Services,
    pub(crate) clock: Arc<TestClock>,
    pub(crate) roots: Vec<PathBuf>,
}

impl Rig {
    pub(crate) fn new(repositories: &[(&str, bool)], cx: &mut TestAppContext) -> Self {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let mut roots = Vec::new();
        for (name, ssh) in repositories {
            let root = fixture.create_repository(name);
            if *ssh {
                fixture.publish_over_ssh(&root);
            }
            fixture.refresh(&root);
            roots.push(root);
        }
        Self::over(fixture, roots, cx)
    }

    /// The window over `fixture`, whose repositories `roots` the caller made
    /// (and published) in it, with a clock moved by hand.
    pub(crate) fn over(fixture: Fixture, roots: Vec<PathBuf>, cx: &mut TestAppContext) -> Self {
        let shell = Shell::open(fixture, cx);
        let clock = Arc::new(TestClock::at(OffsetDateTime::now_utc()));
        let services = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().clone());
        let services = cx.update(|cx| services.with_clock(clock.clone(), cx));
        Self {
            shell,
            services,
            clock,
            roots,
        }
    }

    pub(crate) fn key(&self, ix: usize) -> String {
        key(&self.roots[ix])
    }

    /// `ix`'s repository enters the schedule, enabled, unpaused, with an
    /// interval of 300 s.
    pub(crate) fn schedule(&self, ix: usize, cx: &mut TestAppContext) {
        self.policy(ix, policy(), cx);
    }

    pub(crate) fn policy(&self, ix: usize, policy: Policy, cx: &mut TestAppContext) {
        let root = self.key(ix);
        self.services.scheduler().update(cx, |scheduler, cx| {
            scheduler.schedule_mut().add(root, policy, None);
            cx.notify();
        });
        self.shell.run(cx);
    }

    /// `ix`'s repository answered `answer`.
    pub(crate) fn answer(&self, ix: usize, answer: Answer, cx: &mut TestAppContext) {
        let root = self.key(ix);
        let now = self.clock.now();
        let attempt = PollAttempt {
            root: root.clone(),
            operation: OperationId::new(),
            completed_at: (answer == Answer::Completed).then_some(now.utc),
            answer,
            next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
            targets: Vec::new(),
            index_pending: false,
        };
        self.services.scheduler().update(cx, |scheduler, cx| {
            scheduler.schedule_mut().after(&now, &attempt);
            cx.notify();
        });
        self.shell.run(cx);
    }

    /// Opens the page by keyboard and click, and waits for its rows.
    pub(crate) async fn open(&self, rows: usize, cx: &mut TestAppContext) {
        self.shell.keys("secondary-,", cx);
        self.shell.click(("settings-page", 2usize), cx);
        let last = rows.saturating_sub(1);
        self.shell
            .until_drawn(cx, "the page's rows", move |shell, cx| {
                shell.present(("polling-name", last), cx)
            })
            .await;
    }

    /// The row of the repository named `name`.
    pub(crate) fn row(&self, name: &str, cx: &mut TestAppContext) -> usize {
        self.shell
            .rows("polling-name", cx)
            .iter()
            .position(|shown| shown == name)
            .unwrap_or_else(|| panic!("no row for {name}"))
    }

    pub(crate) fn status(&self, ix: usize, cx: &mut TestAppContext) -> Option<String> {
        self.shell.label(("polling-status", ix), cx)
    }

    /// The requests of the user lane that `pick` picks, oldest first.
    pub(crate) fn sent(
        &self,
        cx: &mut TestAppContext,
        pick: impl Fn(&Intent) -> bool,
    ) -> Vec<DesktopRequest> {
        let operations = self.shell.operations(cx);
        let mut sent: Vec<DesktopRequest> = operations.read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter(|request| pick(&request.intent))
                .cloned()
                .collect()
        });
        sent.reverse();
        sent
    }
}

pub(crate) fn policy() -> Policy {
    Policy {
        enabled: true,
        paused: false,
        interval: Duration::from_secs(300),
        backoff: None,
        recovery_suspended: false,
        next_eligible_at: None,
        active_operation: None,
    }
}
