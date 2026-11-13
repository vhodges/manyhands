//! The needs-attention tests' rig (D6 Task 8): the main window over the
//! polling double, polling switched on, the clock in the test's hand. A
//! locked key, a host and the rest reach the scheduler as the transport
//! would give them (`fail_poll`, `ask_unlock`); what an action sends is
//! read from the requests the service received.

use std::{path::PathBuf, sync::Arc};

use gpui_kit::TestAppContext;
use manyhands::repository::{SharedKeyId, keys::UnlockRequest};
use time::OffsetDateTime;

use crate::desktop::{
    model::polling::{Reason, TestClock},
    services::{
        Services,
        requests::{DesktopRequest, Intent},
    },
    test_support::{
        fixtures::Fixture,
        polling::{PollingBackend, unlock_request},
    },
    view::harness::{Shell, key},
};

pub(super) struct Rig {
    pub shell: Shell,
    pub services: Services,
    pub backend: Arc<PollingBackend>,
    pub clock: Arc<TestClock>,
    pub roots: Vec<String>,
    keys: tempfile::TempDir,
}

impl Rig {
    /// The repositories `repositories` (named, and published over SSH when
    /// the flag is set), polled by nothing until `list`.
    pub(super) fn new(repositories: &[(&str, bool)], cx: &mut TestAppContext) -> Self {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let mut roots = Vec::new();
        for (name, ssh) in repositories {
            let root = fixture.create_repository(name);
            if *ssh {
                fixture.publish_over_ssh(&root);
            }
            fixture.refresh(&root);
            roots.push(key(&root));
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
            keys: tempfile::tempdir().unwrap(),
        }
    }

    /// A key whose passphrase every poll asks the session for, as the
    /// transport does for a protected key.
    pub(super) fn lock(&self) -> UnlockRequest {
        let request = unlock_request(self.keys.path(), SharedKeyId::new());
        self.backend.ask_unlock(request.clone());
        request
    }

    /// A key, for a failure that names one.
    pub(super) fn key(&self) -> SharedKeyId {
        SharedKeyId::new()
    }

    /// The repositories are listed: the scheduler reads them and polls the
    /// due ones.
    pub(super) fn list(&self, cx: &mut TestAppContext) {
        let services = self.services.clone();
        let roots = self.roots.clone();
        cx.update(|cx| services.repositories_listed(&roots, cx));
        self.shell.run(cx);
    }

    /// The scheduler's entries as `(reason, roots)`, roots sorted.
    pub(super) fn entries(&self, cx: &mut TestAppContext) -> Vec<(Reason, Vec<String>)> {
        self.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .attention()
                .into_iter()
                .map(|entry| {
                    let mut roots = entry.roots;
                    roots.sort();
                    (entry.reason, roots)
                })
                .collect()
        })
    }

    /// Waits until the scheduler holds `count` entries.
    pub(super) async fn until_entries(&self, count: usize, cx: &mut TestAppContext) {
        self.shell
            .until_drawn(cx, "the entries", |_, cx| self.entries(cx).len() == count)
            .await;
    }

    /// The requests of the user lane that `pick` picks, oldest first.
    pub(super) fn sent(
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

    /// The Check now requests sent so far, as roots.
    pub(super) fn checks(&self, cx: &mut TestAppContext) -> Vec<PathBuf> {
        self.sent(cx, |intent| matches!(intent, Intent::PollOnce { .. }))
            .into_iter()
            .filter_map(|request| match request.intent {
                Intent::PollOnce { root } => Some(root),
                _ => None,
            })
            .collect()
    }
}
