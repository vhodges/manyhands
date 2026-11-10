//! What the scheduler does with what it hears: a poll's answer and a
//! reconfiguration (D6 design B3, R-10, R-11). See `entity` for the rest.

use std::{collections::BTreeSet, path::PathBuf};

use gpui_kit::Context;

use super::{entity::Scheduler, reconfigure::Scope, registration};
use crate::desktop::{
    model::polling::{CancelCause, PollAttempt, Reason, Root},
    services::{
        requests::{Intent, Lane},
        scheduler::apply,
    },
};

impl Scheduler {
    /// Applies the oldest unapplied answer after a fresh read of the row.
    pub(super) fn next_answer(&mut self, cx: &mut Context<Self>) {
        if self.answering {
            return;
        }
        let Some((root, attempt)) = self.answers.pop_front() else {
            return;
        };
        self.answering = true;
        let backend = self.backend.clone();
        cx.spawn(async move |this, cx| {
            let status = cx
                .background_executor()
                .spawn(async move { registration::status_of(backend.as_ref(), &root) })
                .await;
            let _ = this.update(cx, |scheduler, cx| scheduler.answered(attempt, status, cx));
        })
        .detach();
    }

    /// A poll's answer, with `polling_status` read after it (`None` when
    /// the read failed): the record settles on its `active_operation_id`
    /// and the model moves (R-31, A4).
    pub(super) fn answered(
        &mut self,
        attempt: PollAttempt,
        status: Option<manyhands::repository::PollingStatusDto>,
        cx: &mut Context<Self>,
    ) {
        let now = self.clock.now();
        let launched_at = self.launched_at.unwrap_or(now.utc);
        let root = attempt.root.clone();
        let active = registration::active_of(status.as_ref());
        let summary = apply(
            &mut self.schedule,
            &mut self.store,
            &now,
            &attempt,
            active,
            launched_at,
        );
        // The read just made is the freshest policy there is: a repository
        // the answer left stale takes it at once.
        if self.schedule.repo(&root).is_some_and(|repo| repo.stale)
            && let Some(status) = &status
        {
            self.schedule
                .set_policy(&root, registration::policy_of(status));
        }
        if let Some(summary) = summary {
            if summary.changed() {
                // Every tab of the repository reads again (R-10, Fact 17).
                let changed = root.clone();
                let _ = self
                    .operations
                    .update(cx, |operations, cx| operations.brought_in(changed, cx));
            }
            if summary.index_pending {
                self.refresh_index(&root);
            }
            // TODO(Task 11b): `summary.retire` starts Part E's loop.
        }
        self.answering = false;
        self.publish(cx);
        self.next_answer(cx);
        self.tick(cx);
    }

    /// One background `IndexRefresh` for a poll that completed without its
    /// index refresh (R-10), unless one of the repository is queued.
    fn refresh_index(&self, root: &Root) {
        let root = PathBuf::from(root);
        let queued = self.worker.in_flight(
            |intent| matches!(intent, Intent::IndexRefresh { root: queued } if *queued == root),
        );
        if !queued {
            self.worker
                .submit(Intent::IndexRefresh { root }, Lane::Background, None, None);
        }
    }

    // Reconfiguration (R-11).

    /// The repositories a reconfiguration reaches (`reconfigure`'s rule).
    pub(super) fn reached(&self, scope: &Scope) -> Vec<Root> {
        let mut roots = BTreeSet::new();
        match scope {
            Scope::Repository { root, .. } => {
                roots.insert(root.clone());
            }
            Scope::Host { root, authority } => {
                roots.insert(root.clone());
                // Every scheduled repository reached at that host, and any
                // whose host is not known (an over-approximation costs a
                // cancel and a re-poll).
                roots.extend(
                    self.authorities
                        .iter()
                        .filter(|(_, held)| held.as_ref().is_none_or(|held| held == authority))
                        .map(|(root, _)| root.clone()),
                );
                for entry in self.schedule.attention() {
                    if matches!(&entry.reason, Reason::HostUnapproved(held) if held == authority) {
                        roots.extend(entry.roots);
                    }
                }
            }
            Scope::Keys => roots.extend(self.known.iter().cloned()),
        }
        roots.into_iter().collect()
    }

    /// The change was submitted: the retirements queued for the repositories
    /// it reaches are dropped (R-11).
    pub(super) fn drop_queued_retirements(&mut self, roots: &[Root]) {
        for root in roots {
            // TODO(Task 11b): also drop the loop's queued items; there is no
            // retirement loop yet.
            self.schedule.end_retiring(root);
        }
    }

    /// The change finished: the polls of the repositories it reaches are
    /// cancelled, their attention is cleared and their policies and remotes
    /// are read again, so they re-arm. A removal drops the schedule. A
    /// stored pause is the library's, so a re-added remote leaves it alone.
    pub(super) fn reconfigured(&mut self, scope: Scope, succeeded: bool, cx: &mut Context<Self>) {
        let reached = self.reached(&scope);
        let gone = match &scope {
            Scope::Repository { root, gone: true } => Some(root.clone()),
            _ => None,
        };
        let in_flight = reached.iter().any(|root| {
            self.schedule
                .repo(root)
                .is_some_and(|repo| repo.in_flight())
        });
        if in_flight {
            // It answers `Cancelled(Reconfiguration)`. A poll still queued
            // is not running to cancel; it answers whatever it finds, and
            // the policy, stale, is read again after it.
            self.worker.ask_poll_cancel(CancelCause::Reconfiguration);
        }
        for root in &reached {
            self.schedule.reconfigure(root);
        }
        if let Some(root) = &gone {
            self.forget(root);
        }
        if succeeded {
            self.send_awaited_checks(&reached, gone.as_ref());
        }
        let reread = reached
            .into_iter()
            .filter(|root| Some(root) != gone.as_ref())
            .collect();
        self.read_registrations(reread, cx);
    }
}
