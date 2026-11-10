//! What the person does about a needs-attention entry, at the scheduler
//! (D6 design C3): Unlock…, the Check now that follows a change the person
//! was sent to make, Resume checking, and what a Check now's success
//! proves.
//!
//! An entry is cleared only by proof or by a change (R-11): a Check now of
//! one of its repositories that completed, or a reconfiguration that
//! reached it. Unlock… itself clears nothing; a failed unlock, or an
//! unreachable repository, leaves the entry (K-26). A Check now that
//! fails is an answer like a poll's (A4): it becomes the attention its
//! code names, in place of the entry the repository had.

use std::path::PathBuf;

use gpui_kit::Context;

use super::entity::Scheduler;
use manyhands::{repository::transport::SshAuthority, results::ResultCode};

use crate::desktop::{
    model::{
        operations::Operations,
        polling::{Answer, Authority, KeyId, Phase, Reason, Root, TransportFailure},
    },
    services::{
        poll_convert::check_now_failure,
        requests::{DesktopRequestId, Intent, Lane},
    },
};

impl Scheduler {
    /// The person acted: every repository held for `reason` polls again, at
    /// its stored deadline. Their policies are read again (Resume checking
    /// needs this: the hold is also the policy's flag). Pause is untouched
    /// (A3).
    pub(crate) fn clear_attention(&mut self, reason: &Reason, cx: &mut Context<Self>) {
        let cleared = self.schedule.clear_attention(reason);
        self.read_registrations(cleared, cx);
        self.tick(cx);
    }

    /// A Check now of `root` completed: a completed poll clears the
    /// attention of its repository however the cause was fixed, and, since
    /// it got past authentication, a key's or host's entry for every
    /// repository that shares it (C3). Its policy is read again.
    pub(super) fn checked(&mut self, root: &Root, cx: &mut Context<Self>) {
        let mut reread = vec![root.clone()];
        if let Some(Phase::Attention(reason)) = self.schedule.repo(root).map(|repo| &repo.phase) {
            let reason = reason.clone();
            for cleared in self.schedule.clear_attention(&reason) {
                if !reread.contains(&cleared) {
                    reread.push(cleared);
                }
            }
        }
        self.read_registrations(reread, cx);
    }

    /// A Check now of `root` finished with `code` (C3, A4): what the code
    /// says becomes the repository's attention, as a poll's answer would,
    /// replacing the entry it had (a host answer after the key was
    /// supplied). A repository already held back stays so when the answer
    /// is only that it cannot be reached (K-26); one that is not is `Idle`
    /// on its stored deadline. A code A4 has no row for changes nothing
    /// but the policy read again.
    pub(super) fn check_failed(
        &mut self,
        root: &Root,
        request: DesktopRequestId,
        code: ResultCode,
        authority: Option<Authority>,
        cx: &mut Context<Self>,
    ) {
        let held = match self.schedule.repo(root).map(|repo| &repo.phase) {
            Some(Phase::Attention(reason)) => Some(reason.clone()),
            _ => None,
        };
        let asked = self.worker.asked_keys(request);
        let key = match (asked.as_slice(), &held) {
            ([only], _) => Some(*only),
            (_, Some(Reason::KeyLocked(key) | Reason::KeyProblem(key))) => Some(*key),
            _ => None,
        };
        let authority = authority.map(|authority| SshAuthority {
            host: authority.host,
            port: authority.port,
        });
        let in_flight = self
            .schedule
            .repo(root)
            .is_some_and(|repo| repo.in_flight());
        if let Some(attempt) = check_now_failure(root, code, key, authority)
            && !in_flight
        {
            let unreachable = matches!(
                attempt.answer,
                Answer::Transport(TransportFailure::Unreachable | TransportFailure::Unavailable)
            );
            if !(held.is_some() && unreachable) {
                // The attempt sets the phase, so a retirement loop of the
                // repository ends first, with its mark: it must not go on
                // into a repository that waits (R-32).
                self.end_loop(root, cx);
                let now = self.clock.now();
                self.schedule.after(&now, &attempt);
            }
        }
        self.read_registrations(vec![root.clone()], cx);
    }

    /// An armed Check now whose repository is no longer held back is
    /// dropped: its entry cleared by other means, or the repository went
    /// (a removed and created one starts afresh).
    pub(super) fn prune_arms(&mut self) {
        let schedule = &self.schedule;
        self.check_after.retain(|root| {
            schedule
                .repo(root)
                .is_some_and(|repo| matches!(repo.phase, Phase::Attention(_)))
        });
    }

    /// The repository Unlock… sends its Check now through for `key`: the
    /// first of the key's repositories whose next step reaches
    /// authentication, that is, one that is on, not held and has no local
    /// operation pending (a poll there answers `recovery_required` first,
    /// Fact 32). `None` leaves Unlock… disabled, and the entry pointing at
    /// the pending rows.
    pub(crate) fn unlock_target(&self, key: KeyId, operations: &Operations) -> Option<Root> {
        let held = Reason::KeyLocked(key);
        let entry = self
            .schedule
            .attention()
            .into_iter()
            .find(|entry| entry.reason == held)?;
        entry.roots.into_iter().find(|root| {
            self.schedule
                .repo(root)
                .is_some_and(|repo| repo.policy.enabled && !repo.policy.recovery_suspended)
                && !operations.has_pending_local(root)
        })
    }

    /// Unlock…: clears the key's declined mark, so the prompt can come, and
    /// submits a user-lane Check now through the key's first reachable
    /// repository, which prompts through the existing modal (C3). True when
    /// the Check now was sent; false when no repository reaches
    /// authentication, or that repository's own Check now is still out.
    pub(crate) fn unlock(&mut self, key: KeyId, cx: &mut Context<Self>) -> bool {
        // Nothing starts once quitting is asked (R-33).
        if self.quitting {
            return false;
        }
        let target = self
            .operations
            .read_with(cx, |operations, _| self.unlock_target(key, operations))
            .ok()
            .flatten();
        let Some(root) = target else {
            return false;
        };
        let path = PathBuf::from(&root);
        let out = self
            .worker
            .in_flight(|intent| matches!(intent, Intent::PollOnce { root } if *root == path));
        if out {
            return false;
        }
        self.worker.clear_declined(key);
        self.worker
            .submit(Intent::PollOnce { root: path }, Lane::User, None, None);
        cx.notify();
        true
    }

    /// Resume checking (decision 10): the library's hold is released on a
    /// background task, never the UI thread (R-13). Once it has, the
    /// repository's `RecoveryHeld` attention is cleared and its policy read
    /// again, so it polls at its stored deadline. A call that fails leaves
    /// the hold.
    pub(crate) fn resume_checking(&mut self, root: Root, cx: &mut Context<Self>) {
        // Nothing starts once quitting is asked (R-33). One already out
        // finishes: `resumed` frees the repository, and `tick` polls it
        // once quitting is cancelled.
        if self.quitting {
            return;
        }
        let backend = self.backend.clone();
        let path = PathBuf::from(&root);
        cx.spawn(async move |this, cx| {
            let released = cx
                .background_executor()
                .spawn(async move { backend.resume_polling(&path).is_ok() })
                .await;
            let _ = this.update(cx, |scheduler, cx| scheduler.resumed(root, released, cx));
        })
        .detach();
    }

    fn resumed(&mut self, root: Root, released: bool, cx: &mut Context<Self>) {
        if !released {
            return;
        }
        let held = Reason::RecoveryHeld(root.clone());
        let attended = self
            .schedule
            .attention()
            .iter()
            .any(|entry| entry.reason == held);
        self.clear_attention(&held, cx);
        if !attended {
            // Held by the library's flag alone (a poll was running when it
            // was read): the policy is read again all the same.
            self.read_registrations(vec![root], cx);
        }
    }

    /// Choose a key…, Approve host… and Open remote settings… send the
    /// person to the page that fixes the cause; Check now for `root`
    /// follows when the change finishes (C3). Armed once per press.
    pub(crate) fn check_once_changed(&mut self, root: Root) {
        self.check_after.insert(root);
    }

    /// A reconfiguration that reached `reached` finished: the Check now
    /// each armed repository was waiting for is sent on the user lane. A
    /// repository the change removed (`gone`) is not asked. Once quitting is
    /// asked none is sent (R-33); the arm goes with the ask, so Cancel does
    /// not send it later, after the change it waited for is long over.
    pub(super) fn send_awaited_checks(&mut self, reached: &[Root], gone: Option<&Root>) {
        for root in reached {
            if Some(root) != gone && self.check_after.remove(root) && !self.quitting {
                self.worker.submit(
                    Intent::PollOnce {
                        root: PathBuf::from(root),
                    },
                    Lane::User,
                    None,
                    None,
                );
            }
        }
    }
}
